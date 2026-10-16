//! Phase 2 spike (throwaway): seek, crossfade, track-change gap and spectrum capture with kira.
//!
//! `cargo run -p player --release --example spike -- <seek|gap|crossfade|spectrum> <audio files...>`
//! Plays through the default output device, so keep the volume low.

use std::sync::Arc;
use std::sync::atomic::{AtomicU32, AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use kira::effect::{Effect, EffectBuilder};
use kira::sound::PlaybackState;
use kira::sound::streaming::StreamingSoundData;
use kira::{AudioManager, AudioManagerSettings, Decibels, DefaultBackend, Frame, StartTime, Tween};
use realfft::RealFftPlanner;

const RING: usize = 1 << 17;
const SILENCE: f32 = 1e-5;

/// Lock-free mono ring buffer: the audio thread writes, the analysis side reads.
struct Ring {
    data: Vec<AtomicU32>,
    written: AtomicUsize,
    sample_rate: AtomicU32,
}

struct Tap(Arc<Ring>);

impl Effect for Tap {
    fn init(&mut self, sample_rate: u32, _buffer_size: usize) {
        self.0.sample_rate.store(sample_rate, Ordering::Relaxed);
    }

    // The tap sits on the main track, so it sees the mix of everything, crossfades included.
    fn process(&mut self, input: &mut [Frame], _dt: f64, _info: &kira::info::Info) {
        let mut at = self.0.written.load(Ordering::Relaxed);
        for frame in input.iter() {
            let mono = (frame.left + frame.right) * 0.5;
            self.0.data[at % RING].store(mono.to_bits(), Ordering::Relaxed);
            at += 1;
        }
        self.0.written.store(at, Ordering::Release);
    }
}

struct TapBuilder(Arc<Ring>);

impl EffectBuilder for TapBuilder {
    type Handle = ();

    fn build(self) -> (Box<dyn Effect>, ()) {
        (Box::new(Tap(self.0)), ())
    }
}

struct Reader {
    ring: Arc<Ring>,
    read: usize,
}

impl Reader {
    /// New samples since the last call (older ones are dropped if the reader fell a full ring behind).
    fn drain(&mut self) -> Vec<f32> {
        let written = self.ring.written.load(Ordering::Acquire);
        self.read = self.read.max(written.saturating_sub(RING));
        let out = (self.read..written)
            .map(|i| f32::from_bits(self.ring.data[i % RING].load(Ordering::Relaxed)))
            .collect();
        self.read = written;
        out
    }

    fn sample_rate(&self) -> f32 {
        self.ring.sample_rate.load(Ordering::Relaxed) as f32
    }
}

fn start() -> (AudioManager, Reader) {
    let ring = Arc::new(Ring {
        data: (0..RING).map(|_| AtomicU32::new(0)).collect(),
        written: AtomicUsize::new(0),
        sample_rate: AtomicU32::new(0),
    });
    let mut settings = AudioManagerSettings::<DefaultBackend>::default();
    settings
        .main_track_builder
        .add_effect(TapBuilder(ring.clone()));
    let manager = AudioManager::new(settings).expect("audio device");
    // Skip whatever was written before the experiment starts.
    let read = ring.written.load(Ordering::Acquire);
    (manager, Reader { ring, read })
}

fn sound(path: &str) -> StreamingSoundData<kira::sound::FromFileError> {
    StreamingSoundData::from_file(path).unwrap_or_else(|e| panic!("{path}: {e}"))
}

fn ms(n: u64) -> Duration {
    Duration::from_millis(n)
}

fn tween(millis: u64) -> Tween {
    Tween {
        duration: ms(millis),
        ..Default::default()
    }
}

fn rms(samples: &[f32]) -> f32 {
    (samples.iter().map(|s| s * s).sum::<f32>() / samples.len().max(1) as f32).sqrt()
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (mode, files) = args.split_first().expect("mode and files");
    match mode.as_str() {
        "seek" => files.iter().for_each(|f| seek(f)),
        "gap" => gap(&files[0], &files[1]),
        "lead" => files.iter().for_each(|f| lead(f)),
        "tail" => files.iter().for_each(|f| tail(f)),
        "crossfade" => crossfade(&files[0], &files[1]),
        "spectrum" => spectrum(&files[0]),
        other => panic!("unknown mode {other}"),
    }
}

/// Seeks a streaming sound and measures how long until the position and the output follow.
fn seek(path: &str) {
    let (mut manager, mut reader) = start();
    let data = sound(path);
    let length = data.duration().as_secs_f64();
    let target = length / 2.0;
    let mut handle = manager.play(data).expect("play");
    std::thread::sleep(ms(500));
    reader.drain();

    let t0 = Instant::now();
    handle.seek_to(target);
    let mut position_ms = None;
    while t0.elapsed() < ms(2000) {
        if position_ms.is_none() && handle.position() >= target {
            position_ms = Some(t0.elapsed().as_millis());
        }
        std::thread::sleep(ms(1));
    }
    let after = reader.drain();
    println!(
        "seek {path}\n  length {length:.1}s, seek to {target:.1}s: position reached after {:?} ms, position now {:.2}s, output rms {:.3}, state {:?}",
        position_ms,
        handle.position(),
        rms(&after),
        handle.state(),
    );
}

/// Time from `play` to the first audible sample: the track's own leading silence plus start-up latency.
fn lead(path: &str) {
    let (mut manager, mut reader) = start();
    std::thread::sleep(ms(200));
    reader.drain();
    let data = sound(path);
    let _handle = manager.play(data).expect("play");
    let mut silent = 0usize;
    let t0 = Instant::now();
    'wait: while t0.elapsed() < ms(3000) {
        for s in reader.drain() {
            if s.abs() >= SILENCE {
                break 'wait;
            }
            silent += 1;
        }
        std::thread::sleep(ms(1));
    }
    println!(
        "lead {path}\n  silence before the first sample: {:.1} ms (wall time to detect: {:?})",
        silent as f32 * 1000.0 / reader.sample_rate(),
        t0.elapsed()
    );
}

/// Seeks to 2 s before the reported end and prints position, state and output level for 4 s.
fn tail(path: &str) {
    let (mut manager, mut reader) = start();
    let data = sound(path);
    let length = data.duration().as_secs_f64();
    let mut handle = manager.play(data).expect("play");
    std::thread::sleep(ms(300));
    handle.seek_to(length - 2.0);
    let mut line = String::new();
    for _ in 0..8 {
        std::thread::sleep(ms(500));
        line.push_str(&format!(
            "[pos {:.2} {:?} rms {:.3}] ",
            handle.position(),
            handle.state(),
            rms(&reader.drain())
        ));
    }
    println!("tail {path}\n  reported length {length:.2}s\n  {line}");
}

/// Starts the next track when the current one ends and measures the silence in the output.
fn gap(a: &str, b: &str) {
    for strategy in ["poll-stopped", "delayed-start"] {
        let (mut manager, mut reader) = start();
        let data = sound(a);
        let length = data.duration().as_secs_f64();
        let mut first = manager.play(data).expect("play a");
        if length > 10.0 {
            first.seek_to(length - 1.5);
        }

        let mut runs = Vec::new(); // (start in s, length in ms)
        let (mut seen_audio, mut run_start, mut run_len, mut n) = (false, 0usize, 0usize, 0usize);
        let mut scan = |samples: &[f32], sr: f32| {
            for &s in samples {
                if s.abs() < SILENCE {
                    if run_len == 0 {
                        run_start = n;
                    }
                    run_len += 1;
                } else {
                    if seen_audio && run_len >= 8 {
                        runs.push((run_start as f32 / sr, run_len as f32 * 1000.0 / sr));
                    }
                    seen_audio = true;
                    run_len = 0;
                }
                n += 1;
            }
        };

        let t0 = Instant::now();
        let mut second = None;
        let mut started_at = None;
        while t0.elapsed() < ms(5500) {
            if second.is_none() {
                match strategy {
                    "poll-stopped" if first.state() == PlaybackState::Stopped => {
                        second = Some(manager.play(sound(b)).expect("play b"));
                    }
                    "delayed-start" if first.position() > length - 0.4 => {
                        // The position is only as fresh as the last audio callback.
                        let remaining = (length - first.position()).max(0.0);
                        second = Some(
                            manager
                                .play(sound(b).start_time(StartTime::Delayed(
                                    Duration::from_secs_f64(remaining),
                                )))
                                .expect("play b"),
                        );
                    }
                    _ => {}
                }
            }
            if second.is_some() && started_at.is_none() {
                started_at = Some(t0.elapsed());
            }
            let sr = reader.sample_rate();
            scan(&reader.drain(), sr);
            std::thread::sleep(ms(1));
        }
        println!(
            "gap [{strategy}] next track requested at {:?}, first state {:?}, trailing silence {:.1} ms, silent runs of 8+ samples (start s, length ms):",
            started_at,
            first.state(),
            run_len as f32 * 1000.0 / reader.sample_rate(),
        );
        for (start, len) in runs.iter().take(8) {
            println!("  {start:.3}s  {len:.1} ms");
        }
        if runs.is_empty() {
            println!("  none");
        }
    }
}

/// Overlaps two tracks and prints the output level of the mix, with two fade curves.
fn crossfade(a: &str, b: &str) {
    for manual in [false, true] {
        let (mut manager, mut reader) = start();
        let mut first = manager.play(sound(a)).expect("play a");
        let t0 = Instant::now();
        let mut second = None;
        let (mut line, mut next_print) = (String::new(), 0u64);
        while t0.elapsed() < ms(9000) {
            let t = ((t0.elapsed().as_secs_f32() - 3.0) / 3.0).clamp(0.0, 1.0);
            if second.is_none() && t0.elapsed() > ms(3000) {
                if manual {
                    second = Some(
                        manager
                            .play(sound(b).volume(Decibels::SILENCE))
                            .expect("play b"),
                    );
                } else {
                    first.stop(tween(3000));
                    second = Some(
                        manager
                            .play(sound(b).fade_in_tween(tween(3000)))
                            .expect("play b"),
                    );
                }
            }
            if manual && let Some(second) = second.as_mut() {
                // Equal power: the summed energy of two uncorrelated tracks stays constant.
                let db = |amp: f32| Decibels((20.0 * amp.max(1e-3).log10()).max(-60.0));
                let angle = t * std::f32::consts::FRAC_PI_2;
                first.set_volume(db(angle.cos()), tween(20));
                second.set_volume(db(angle.sin()), tween(20));
                if t >= 1.0 {
                    first.stop(tween(20));
                }
            }
            std::thread::sleep(ms(20));
            if t0.elapsed().as_millis() as u64 >= next_print {
                next_print += 250;
                let level = rms(&reader.drain());
                line.push_str(&format!("{:.2} ", level));
            } else {
                let _ = reader.drain();
            }
        }
        println!(
            "crossfade [{}] rms every 250 ms (fade from 3.0s to 6.0s = entries 12 to 24):\n  {line}",
            if manual {
                "manual equal-power"
            } else {
                "kira tween, dB-linear"
            }
        );
    }
}

/// Takes the latest 2048 samples every ~16 ms, runs an FFT and folds it into 64 log bands.
fn spectrum(path: &str) {
    const WINDOW: usize = 2048;
    const BANDS: usize = 64;
    let (mut manager, mut reader) = start();
    let _handle = manager.play(sound(path)).expect("play");
    std::thread::sleep(ms(300));
    reader.drain();

    let mut planner = RealFftPlanner::<f32>::new();
    let fft = planner.plan_fft_forward(WINDOW);
    let mut input = fft.make_input_vec();
    let mut output = fft.make_output_vec();
    let mut scratch = fft.make_scratch_vec();
    let hann: Vec<f32> = (0..WINDOW)
        .map(|i| 0.5 - 0.5 * (std::f32::consts::TAU * i as f32 / (WINDOW - 1) as f32).cos())
        .collect();

    let (mut latest, mut busy, mut frames, mut worst) =
        (Vec::new(), Duration::ZERO, 0u32, Duration::ZERO);
    let mut bands = [0.0f32; BANDS];
    let t0 = Instant::now();
    while t0.elapsed() < ms(5000) {
        std::thread::sleep(ms(16));
        latest.extend(reader.drain());
        if latest.len() > WINDOW {
            latest.drain(..latest.len() - WINDOW);
        }
        if latest.len() < WINDOW {
            continue;
        }
        let work = Instant::now();
        for (slot, (s, w)) in input.iter_mut().zip(latest.iter().zip(&hann)) {
            *slot = s * w;
        }
        fft.process_with_scratch(&mut input, &mut output, &mut scratch)
            .unwrap();
        let bin_hz = reader.sample_rate() / WINDOW as f32;
        let (low, high) = (30.0f32, 16000.0f32);
        bands.fill(0.0);
        for (bin, c) in output.iter().enumerate().skip(1) {
            let hz = bin as f32 * bin_hz;
            if !(low..high).contains(&hz) {
                continue;
            }
            let band = ((hz / low).ln() / (high / low).ln() * BANDS as f32) as usize;
            bands[band.min(BANDS - 1)] = bands[band.min(BANDS - 1)].max(c.norm() / WINDOW as f32);
        }
        let spent = work.elapsed();
        busy += spent;
        worst = worst.max(spent);
        frames += 1;
    }
    let loudest = bands.iter().cloned().fold(0.0, f32::max);
    println!(
        "spectrum {path}\n  {frames} frames in 5 s, avg {:.0} us, worst {:.0} us per frame, busy {:.2}% of one core; last frame max band {loudest:.4}",
        busy.as_micros() as f64 / frames.max(1) as f64,
        worst.as_micros() as f64,
        busy.as_secs_f64() / 5.0 * 100.0,
    );
    println!("  sample rate {} Hz", reader.sample_rate());
}
