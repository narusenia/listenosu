use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{ChildStdin, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use crate::protocol::{Message, QUIT_COMMAND, REFRESH_COMMAND, open_command};

#[derive(Debug)]
pub enum Event {
    Message(Message),
    /// The helper kept failing and restarting was given up; the app falls back to its cache.
    Failed(String),
}

#[derive(Clone, Copy, Debug)]
pub struct Policy {
    /// Restarts in a row without a successful read before giving up.
    pub max_restarts: u32,
    /// Wait before restart number n is `backoff * n`.
    pub backoff: Duration,
}

impl Default for Policy {
    fn default() -> Self {
        Self {
            max_restarts: 3,
            backoff: Duration::from_secs(1),
        }
    }
}

type SharedStdin = Arc<Mutex<Option<ChildStdin>>>;

/// The helper child process, restarted when it dies. Dropping this asks it to quit.
pub struct Helper {
    events: Receiver<Event>,
    stdin: SharedStdin,
    stop: Arc<AtomicBool>,
}

impl Helper {
    pub fn start(exe: PathBuf, data_dir: PathBuf) -> Self {
        Self::start_with(exe, data_dir, Policy::default())
    }

    pub fn start_with(exe: PathBuf, data_dir: PathBuf, policy: Policy) -> Self {
        let (tx, events) = channel();
        let stdin: SharedStdin = Arc::default();
        let stop = Arc::new(AtomicBool::new(false));
        let (shared, flag) = (stdin.clone(), stop.clone());
        thread::spawn(move || supervise(&exe, &data_dir, policy, &tx, &shared, &flag));
        Self {
            events,
            stdin,
            stop,
        }
    }

    pub fn events(&self) -> &Receiver<Event> {
        &self.events
    }

    /// Asks the helper to read the library again; the result arrives as `Changed`.
    pub fn refresh(&self) {
        self.send(REFRESH_COMMAND);
    }

    fn send(&self, line: &str) {
        if let Some(stdin) = self.stdin.lock().unwrap().as_mut() {
            // A dead helper is the supervisor's business; a failed write is not an error here.
            let _ = writeln!(stdin, "{line}").and_then(|()| stdin.flush());
        }
    }
}

impl Drop for Helper {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        self.send(QUIT_COMMAND);
        *self.stdin.lock().unwrap() = None;
    }
}

fn supervise(
    exe: &Path,
    data_dir: &Path,
    policy: Policy,
    tx: &Sender<Event>,
    stdin: &SharedStdin,
    stop: &AtomicBool,
) {
    let mut failures = 0;
    while !stop.load(Ordering::Relaxed) {
        let reason = match run_once(exe, data_dir, tx, stdin) {
            Ok(true) => {
                failures = 0;
                "the helper exited".to_owned()
            }
            Ok(false) => "the helper exited before sending the library".to_owned(),
            Err(e) => format!("could not run the helper: {e}"),
        };
        if stop.load(Ordering::Relaxed) {
            return;
        }
        failures += 1;
        if failures > policy.max_restarts {
            let _ = tx.send(Event::Failed(reason));
            return;
        }
        thread::sleep(policy.backoff * failures);
    }
}

/// Runs one helper process until it exits. True when it delivered the library at least once.
fn run_once(
    exe: &Path,
    data_dir: &Path,
    tx: &Sender<Event>,
    shared_stdin: &SharedStdin,
) -> std::io::Result<bool> {
    let mut command = Command::new(exe);
    command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x0800_0000); // CREATE_NO_WINDOW: no console flashing up
    }
    let mut child = command.spawn()?;

    let mut stdin = child.stdin.take().expect("stdin is piped");
    writeln!(stdin, "{}", open_command(data_dir))?;
    *shared_stdin.lock().unwrap() = Some(stdin);

    let mut delivered = false;
    let stdout = child.stdout.take().expect("stdout is piped");
    for line in BufReader::new(stdout).lines() {
        let Ok(line) = line else { break };
        // A line that is not a known message is ignored: a newer helper may add message types.
        let Ok(message) = serde_json::from_str::<Message>(&line) else {
            continue;
        };
        delivered |= matches!(message, Message::Snapshot { .. } | Message::Changed { .. });
        if tx.send(Event::Message(message)).is_err() {
            break; // the app dropped its receiver
        }
    }
    *shared_stdin.lock().unwrap() = None;
    let _ = child.kill();
    let _ = child.wait();
    Ok(delivered)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gives_up_after_the_restart_limit_when_the_helper_cannot_run() {
        let policy = Policy {
            max_restarts: 2,
            backoff: Duration::from_millis(5),
        };
        let helper = Helper::start_with(
            PathBuf::from("listenosu-helper-does-not-exist"),
            PathBuf::from("."),
            policy,
        );
        let event = helper
            .events()
            .recv_timeout(Duration::from_secs(5))
            .unwrap();
        assert!(matches!(event, Event::Failed(reason) if reason.contains("could not run")));
    }

    /// Needs the built helper and a lazer library:
    /// `cargo test -p library -- --ignored` after `dotnet build helper/Helper -c Release`.
    #[test]
    #[ignore]
    fn reads_the_real_library_through_the_real_helper() {
        let root =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../helper/Helper/bin/Release/net10.0");
        let exe = root.join(if cfg!(windows) {
            "listenosu-helper.exe"
        } else {
            "listenosu-helper"
        });
        let data_dir = crate::discover::detect().expect("a lazer folder");
        let helper = Helper::start(exe, data_dir);
        let mut songs = None;
        while let Ok(event) = helper.events().recv_timeout(Duration::from_secs(20)) {
            if let Event::Message(Message::Snapshot { songs: s, .. }) = event {
                songs = Some(s);
                break;
            }
        }
        let songs = songs.expect("a snapshot within 20 s");
        assert!(!songs.is_empty());
        assert!(songs.iter().all(|s| !s.audio_hash.is_empty()));
    }
}
