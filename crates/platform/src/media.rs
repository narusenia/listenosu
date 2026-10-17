use std::ffi::c_void;
use std::path::Path;
use std::sync::mpsc::{Receiver, channel};
use std::time::{Duration, Instant};

use souvlaki::{
    MediaControlEvent, MediaControls, MediaMetadata, MediaPlayback, MediaPosition, PlatformConfig,
    SeekDirection,
};

/// How far the OS-side clock may drift from the real position before it is told again.
const DRIFT: f64 = 1.0;
/// Step for a seek key that carries no amount.
const SEEK_STEP: f64 = 5.0;

/// A request from the OS media keys or the system media overlay.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Command {
    Play,
    Pause,
    Toggle,
    Next,
    Previous,
    Stop,
    /// Jump to an absolute position in seconds.
    SeekTo(f64),
    /// Move by a signed number of seconds.
    SeekBy(f64),
}

#[derive(Clone, Copy, PartialEq)]
enum State {
    Stopped,
    Paused,
    Playing,
}

/// The song shown in the OS media overlay.
pub struct Track<'a> {
    pub title: &'a str,
    pub artist: &'a str,
    pub cover: Option<&'a Path>,
    pub length_secs: f64,
}

/// SMTC on Windows, Now Playing on macOS, MPRIS on Linux.
pub struct Media {
    controls: MediaControls,
    commands: Receiver<Command>,
    sent: Option<(State, f64, Instant)>,
}

impl Media {
    /// `hwnd` is the main window's handle; it is required on Windows and ignored elsewhere.
    pub fn new(hwnd: Option<*mut c_void>) -> Result<Self, String> {
        let mut controls = MediaControls::new(PlatformConfig {
            display_name: "listenosu!",
            dbus_name: "listenosu",
            hwnd,
        })
        .map_err(|e| format!("{e:?}"))?;
        let (tx, commands) = channel();
        controls
            .attach(move |event| {
                if let Some(command) = command_of(event) {
                    let _ = tx.send(command);
                }
            })
            .map_err(|e| format!("{e:?}"))?;
        Ok(Self {
            controls,
            commands,
            sent: None,
        })
    }

    pub fn poll(&self) -> Vec<Command> {
        self.commands.try_iter().collect()
    }

    pub fn set_track(&mut self, track: Option<&Track>) {
        let cover = track
            .and_then(|t| t.cover)
            .map(|p| format!("file://{}", p.display()));
        let metadata = track.map(|t| MediaMetadata {
            title: Some(t.title),
            artist: Some(t.artist),
            cover_url: cover.as_deref(),
            duration: Some(Duration::from_secs_f64(t.length_secs.max(0.0))),
            ..Default::default()
        });
        let _ = self.controls.set_metadata(metadata.unwrap_or_default());
        // A new song starts a new timeline.
        self.sent = None;
    }

    /// `playing` is None when nothing is loaded. Cheap to call every frame: the OS is only told
    /// on a state change or when the position jumps (seek, new song).
    pub fn set_state(&mut self, playing: Option<bool>, position_secs: f64) {
        let state = match playing {
            None => State::Stopped,
            Some(true) => State::Playing,
            Some(false) => State::Paused,
        };
        if let Some((last, at, when)) = self.sent {
            let expected = if last == State::Playing {
                at + when.elapsed().as_secs_f64()
            } else {
                at
            };
            if last == state && (expected - position_secs).abs() < DRIFT {
                return;
            }
        }
        let progress = Some(MediaPosition(Duration::from_secs_f64(
            position_secs.max(0.0),
        )));
        let _ = self.controls.set_playback(match state {
            State::Stopped => MediaPlayback::Stopped,
            State::Paused => MediaPlayback::Paused { progress },
            State::Playing => MediaPlayback::Playing { progress },
        });
        self.sent = Some((state, position_secs, Instant::now()));
    }
}

fn command_of(event: MediaControlEvent) -> Option<Command> {
    Some(match event {
        MediaControlEvent::Play => Command::Play,
        MediaControlEvent::Pause => Command::Pause,
        MediaControlEvent::Toggle => Command::Toggle,
        MediaControlEvent::Next => Command::Next,
        MediaControlEvent::Previous => Command::Previous,
        MediaControlEvent::Stop => Command::Stop,
        MediaControlEvent::SetPosition(p) => Command::SeekTo(p.0.as_secs_f64()),
        MediaControlEvent::SeekBy(dir, by) => Command::SeekBy(signed(dir, by.as_secs_f64())),
        MediaControlEvent::Seek(dir) => Command::SeekBy(signed(dir, SEEK_STEP)),
        _ => return None,
    })
}

fn signed(direction: SeekDirection, secs: f64) -> f64 {
    match direction {
        SeekDirection::Forward => secs,
        SeekDirection::Backward => -secs,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seek_events_become_signed_steps() {
        assert_eq!(
            command_of(MediaControlEvent::Seek(SeekDirection::Backward)),
            Some(Command::SeekBy(-SEEK_STEP))
        );
        assert_eq!(command_of(MediaControlEvent::Raise), None);
    }
}
