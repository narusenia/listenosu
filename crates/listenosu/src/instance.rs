//! Only one instance runs. A second launch asks the first to show its window, then exits.

use std::fs::{File, TryLockError};
use std::io::{Read, Write};
use std::net::{Ipv4Addr, TcpListener, TcpStream};
use std::time::Duration;

use library::store::AppDirs;

/// What a second instance sends. It only ever asks for the window; nothing else is accepted.
const SHOW: &[u8] = b"listenosu:show";
const TIMEOUT: Duration = Duration::from_secs(1);

/// Held until the process exits; the OS releases the lock even after a crash.
pub struct Instance {
    _lock: Option<File>,
    /// Where a second launch connects. None when no local port could be opened.
    pub wake: Option<TcpListener>,
}

/// Takes the lock that marks this process as the only running instance. `None` means another
/// instance holds it (it has been asked to show its window). A lock that cannot be made at all
/// counts as free, so the app still starts.
pub fn claim(dirs: Option<&AppDirs>) -> Option<Instance> {
    let Some(dirs) = dirs else {
        return Some(Instance {
            _lock: None,
            wake: None,
        });
    };
    let file = std::fs::create_dir_all(&dirs.config)
        .and_then(|()| File::create(dirs.config.join("instance.lock")))
        .ok();
    match file.map(|f| f.try_lock().map(|()| f)) {
        Some(Err(TryLockError::WouldBlock)) => {
            let _ = ask_to_show(dirs);
            None
        }
        Some(Ok(file)) => Some(Instance {
            _lock: Some(file),
            wake: listen(dirs),
        }),
        _ => Some(Instance {
            _lock: None,
            wake: None,
        }),
    }
}

/// Opens a loopback port and records it for the next launch. The port is picked by the OS.
fn listen(dirs: &AppDirs) -> Option<TcpListener> {
    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).ok()?;
    let port = listener.local_addr().ok()?.port();
    std::fs::write(port_file(dirs), port.to_string()).ok()?;
    Some(listener)
}

fn port_file(dirs: &AppDirs) -> std::path::PathBuf {
    dirs.config.join("instance.port")
}

fn ask_to_show(dirs: &AppDirs) -> std::io::Result<()> {
    let port: u16 = std::fs::read_to_string(port_file(dirs))?
        .trim()
        .parse()
        .map_err(std::io::Error::other)?;
    let mut stream = TcpStream::connect_timeout(&(Ipv4Addr::LOCALHOST, port).into(), TIMEOUT)?;
    stream.write_all(SHOW)
}

/// Runs `on_show` on a background thread each time a second launch asks for the window.
pub fn serve(listener: TcpListener, on_show: impl Fn() + Send + 'static) {
    std::thread::spawn(move || {
        for mut stream in listener.incoming().flatten() {
            let _ = stream.set_read_timeout(Some(TIMEOUT));
            let mut message = Vec::new();
            // Cap the read so a stray connection cannot make us buffer without end.
            let _ = (&mut stream).take(64).read_to_end(&mut message);
            if message == SHOW {
                on_show();
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc;

    fn dirs(name: &str) -> AppDirs {
        let dir = std::env::temp_dir().join(format!("listenosu-{name}-{}", std::process::id()));
        AppDirs {
            config: dir.clone(),
            cache: dir,
        }
    }

    #[test]
    fn a_second_claim_is_refused_until_the_first_is_dropped() {
        let dirs = dirs("lock");
        let first = claim(Some(&dirs));
        assert!(first.is_some());
        assert!(claim(Some(&dirs)).is_none());
        drop(first);
        assert!(claim(Some(&dirs)).is_some());
        let _ = std::fs::remove_dir_all(&dirs.config);
    }

    #[test]
    fn a_refused_launch_asks_the_first_instance_to_show_its_window() {
        let dirs = dirs("wake");
        let first = claim(Some(&dirs)).unwrap();
        let (tx, rx) = mpsc::channel();
        serve(
            first.wake.as_ref().unwrap().try_clone().unwrap(),
            move || {
                let _ = tx.send(());
            },
        );
        assert!(claim(Some(&dirs)).is_none());
        assert!(rx.recv_timeout(Duration::from_secs(5)).is_ok());
        let _ = std::fs::remove_dir_all(&dirs.config);
    }
}
