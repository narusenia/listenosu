//! Playback over kira: transport, queue, equal-power crossfade and skipping of unplayable tracks.
//!
//! The player has no thread of its own. The owner calls `tick` often (every UI frame is fine);
//! it moves fades along, detects the end of a track and starts the next one.

use std::path::PathBuf;
use std::time::{Duration, Instant};

use kira::sound::FromFileError;
use kira::sound::PlaybackState;
use kira::sound::streaming::{StreamingSoundData, StreamingSoundHandle};
use kira::{AudioManager, AudioManagerSettings, Decibels, DefaultBackend, Tween};

use crate::queue::{Advance, Queue, Repeat};
use crate::spectrum::{Analyzer, BANDS, Ring, TapBuilder};

type Handle = StreamingSoundHandle<FromFileError>;
type Resolver<T> = Box<dyn Fn(&T) -> Option<PathBuf>>;

/// Fade for a skip the user asked for; automatic track changes use the configured crossfade.
const MANUAL_FADE: Duration = Duration::from_millis(300);
/// Volume steps for fades. kira's own volume tween interpolates in dB and dips in the middle.
const STEP: Duration = Duration::from_millis(20);
/// A track this close to its end counts as ended.
const END_MARGIN: f64 = 0.05;
/// Previous restarts the track instead of going back when it has played this long.
const RESTART_AFTER: f64 = 3.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Event {
    /// Playback moved to this queue index (also after a skip).
    Started(usize),
    /// This queue index could not be played and was skipped.
    Skipped(usize),
    /// The queue ran out.
    Finished,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Status {
    /// Queue index of the current track.
    pub index: Option<usize>,
    pub position: f64,
    /// Decoded length in seconds. Prefer it over the library's `length_ms`, which can be 0.
    pub length: f64,
    pub paused: bool,
}

struct Voice {
    handle: Handle,
    length: f64,
}

struct Fade {
    out: Voice,
    elapsed: Duration,
    total: Duration,
}

pub struct Player<T> {
    manager: AudioManager,
    analyzer: Analyzer,
    queue: Queue<T>,
    resolve: Resolver<T>,
    current: Option<Voice>,
    fade: Option<Fade>,
    volume: f32,
    crossfade: Duration,
    paused: bool,
    last_tick: Instant,
    events: Vec<Event>,
}

fn tween(duration: Duration) -> Tween {
    Tween {
        duration,
        ..Default::default()
    }
}

fn db(amplitude: f32) -> Decibels {
    Decibels((20.0 * amplitude.max(1e-3).log10()).max(-60.0))
}

impl<T> Player<T> {
    /// `resolve` turns a queue item into an audio file path, or `None` when the file is missing.
    pub fn new(resolve: impl Fn(&T) -> Option<PathBuf> + 'static) -> Result<Self, String> {
        let ring = Ring::new();
        let mut settings = AudioManagerSettings::<DefaultBackend>::default();
        settings
            .main_track_builder
            .add_effect(TapBuilder(ring.clone()));
        let mut queue = Queue::default();
        queue.reseed(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(1, |d| d.as_nanos() as u64),
        );
        Ok(Self {
            manager: AudioManager::new(settings).map_err(|e| e.to_string())?,
            analyzer: Analyzer::new(ring),
            queue,
            resolve: Box::new(resolve),
            current: None,
            fade: None,
            volume: 1.0,
            crossfade: Duration::ZERO,
            paused: false,
            last_tick: Instant::now(),
            events: Vec::new(),
        })
    }

    pub fn queue(&self) -> &Queue<T> {
        &self.queue
    }

    pub fn status(&self) -> Status {
        let Some(voice) = &self.current else {
            return Status::default();
        };
        Status {
            index: self.queue.current(),
            position: voice.handle.position().min(voice.length),
            length: voice.length,
            paused: self.paused,
        }
    }

    /// Replaces the queue and starts playing `start`.
    pub fn play(&mut self, items: Vec<T>, start: usize) {
        self.queue.set(items, start);
        if let Some(index) = self.queue.current() {
            self.begin(index, MANUAL_FADE, false);
        }
    }

    /// Replaces the queue and loads `start` paused at `position`, for restoring the last session.
    pub fn restore(&mut self, items: Vec<T>, start: usize, position: f64) {
        self.queue.set(items, start);
        if let Some(index) = self.queue.current() {
            self.begin(index, Duration::ZERO, true);
            self.seek(position);
        }
    }

    /// Plays a queue item now.
    pub fn jump(&mut self, index: usize) {
        if self.queue.jump(index) {
            self.begin(index, MANUAL_FADE, false);
        }
    }

    pub fn pause(&mut self) {
        if self.current.is_some() && !self.paused {
            self.paused = true;
            self.for_each_voice(|h| h.pause(tween(STEP)));
        }
    }

    pub fn resume(&mut self) {
        if self.current.is_some() && self.paused {
            self.paused = false;
            self.for_each_voice(|h| h.resume(tween(STEP)));
        }
    }

    pub fn toggle(&mut self) {
        if self.paused {
            self.resume();
        } else {
            self.pause();
        }
    }

    pub fn seek(&mut self, seconds: f64) {
        if let Some(voice) = &mut self.current {
            voice.handle.seek_to(seconds.clamp(0.0, voice.length));
        }
    }

    pub fn set_volume(&mut self, volume: f32) {
        self.volume = volume.clamp(0.0, 1.0);
        if self.fade.is_none()
            && let Some(voice) = &mut self.current
        {
            voice.handle.set_volume(db(self.volume), tween(STEP));
        }
    }

    pub fn set_crossfade(&mut self, crossfade: Duration) {
        self.crossfade = crossfade;
    }

    pub fn set_shuffle(&mut self, on: bool) {
        self.queue.set_shuffle(on);
    }

    pub fn set_repeat(&mut self, repeat: Repeat) {
        self.queue.set_repeat(repeat);
    }

    /// Adds to the end of the queue without interrupting playback.
    pub fn enqueue(&mut self, item: T) {
        self.queue.push(item);
    }

    /// Removes a queue item. Removing the playing track moves on to the next one.
    pub fn remove(&mut self, index: usize) {
        let playing = self.queue.current() == Some(index) && self.current.is_some();
        self.queue.remove(index);
        if !playing {
            return;
        }
        match self.queue.current() {
            Some(next) => self.begin(next, MANUAL_FADE, false),
            None => {
                self.stop_all();
                self.events.push(Event::Finished);
            }
        }
    }

    pub fn swap(&mut self, a: usize, b: usize) {
        self.queue.swap(a, b);
    }

    /// Stops playback and empties the queue.
    pub fn clear(&mut self) {
        self.queue.clear();
        self.stop_all();
    }

    pub fn next(&mut self) {
        if let Some(index) = self.queue.advance(Advance::Manual) {
            self.begin(index, MANUAL_FADE, false);
        }
    }

    pub fn previous(&mut self) {
        if self.status().position > RESTART_AFTER {
            self.seek(0.0);
        } else if let Some(index) = self.queue.previous() {
            self.begin(index, MANUAL_FADE, false);
        }
    }

    /// Band levels (0.0 to 1.0, low to high) for the spectrum. Pass `visible = false` while the window is hidden:
    /// nothing is analyzed then, and nothing is analyzed while paused either.
    pub fn spectrum(&mut self, visible: bool) -> [f32; BANDS] {
        if !visible || self.paused || self.current.is_none() {
            self.analyzer.skip();
            return [0.0; BANDS];
        }
        *self.analyzer.levels()
    }

    /// Moves fades along, starts the next track when the current one ends, and returns what happened since the last call.
    pub fn tick(&mut self) -> Vec<Event> {
        let now = Instant::now();
        let dt = now - self.last_tick;
        if dt >= STEP {
            self.last_tick = now;
            if !self.paused {
                self.step_fade(dt);
                self.check_end();
            }
        }
        std::mem::take(&mut self.events)
    }

    fn step_fade(&mut self, dt: Duration) {
        let Some(fade) = &mut self.fade else { return };
        fade.elapsed += dt;
        let t = (fade.elapsed.as_secs_f32() / fade.total.as_secs_f32()).clamp(0.0, 1.0);
        // Equal power: the summed energy of two uncorrelated tracks stays constant.
        let angle = t * std::f32::consts::FRAC_PI_2;
        fade.out
            .handle
            .set_volume(db(angle.cos() * self.volume), tween(STEP));
        if let Some(voice) = &mut self.current {
            voice
                .handle
                .set_volume(db(angle.sin() * self.volume), tween(STEP));
        }
        if t >= 1.0 {
            let mut done = self.fade.take().unwrap();
            done.out.handle.stop(tween(STEP));
            if let Some(voice) = &mut self.current {
                voice.handle.set_volume(db(self.volume), tween(STEP));
            }
        }
    }

    fn check_end(&mut self) {
        if self.fade.is_some() {
            return;
        }
        let Some(voice) = &self.current else { return };
        let remaining = voice.length - voice.handle.position();
        // Long tracks can end without ever reporting Stopped, so the position decides.
        let ended = remaining <= END_MARGIN || voice.handle.state() == PlaybackState::Stopped;
        let lead = self.crossfade.as_secs_f64().min(voice.length / 2.0);
        if !ended && remaining > lead {
            return;
        }
        match self.queue.advance(Advance::Auto) {
            Some(index) => {
                let fade = if ended {
                    Duration::ZERO
                } else {
                    Duration::from_secs_f64(lead)
                };
                self.begin(index, fade, false);
            }
            None if ended => {
                self.stop_all();
                self.events.push(Event::Finished);
            }
            None => {}
        }
    }

    /// Starts `index`, skipping forward over tracks that cannot be played, and fades from the current track.
    fn begin(&mut self, mut index: usize, fade: Duration, paused: bool) {
        // A pause is lifted by starting something; a fade from a paused track makes no sense.
        let fade = if self.paused || paused {
            Duration::ZERO
        } else {
            fade
        };
        for _ in 0..self.queue.items().len() {
            let silent = !fade.is_zero() && self.current.is_some();
            if let Some(voice) = self.try_play(index, silent) {
                self.replace_current(voice, fade);
                self.paused = paused;
                if paused && let Some(voice) = &mut self.current {
                    voice.handle.pause(tween(Duration::ZERO));
                }
                self.events.push(Event::Started(index));
                return;
            }
            self.events.push(Event::Skipped(index));
            match self.queue.advance(Advance::Manual) {
                Some(next) => index = next,
                None => break,
            }
        }
        self.stop_all();
        self.events.push(Event::Finished);
    }

    fn try_play(&mut self, index: usize, silent: bool) -> Option<Voice> {
        let path = (self.resolve)(&self.queue.items()[index])?;
        let data = StreamingSoundData::from_file(path).ok()?;
        let length = data.duration().as_secs_f64();
        let volume = if silent {
            Decibels::SILENCE
        } else {
            db(self.volume)
        };
        let handle = self.manager.play(data.volume(volume)).ok()?;
        Some(Voice { handle, length })
    }

    fn replace_current(&mut self, voice: Voice, fade: Duration) {
        if let Some(mut stale) = self.fade.take() {
            stale.out.handle.stop(tween(STEP));
        }
        match self.current.replace(voice) {
            Some(out) if !fade.is_zero() => {
                self.fade = Some(Fade {
                    out,
                    elapsed: Duration::ZERO,
                    total: fade,
                });
            }
            Some(mut out) => out.handle.stop(tween(STEP)),
            None => {}
        }
    }

    fn stop_all(&mut self) {
        self.for_each_voice(|h| h.stop(tween(STEP)));
        self.current = None;
        self.fade = None;
        self.paused = false;
    }

    fn for_each_voice(&mut self, mut f: impl FnMut(&mut Handle)) {
        if let Some(voice) = &mut self.current {
            f(&mut voice.handle);
        }
        if let Some(fade) = &mut self.fade {
            f(&mut fade.out.handle);
        }
    }
}
