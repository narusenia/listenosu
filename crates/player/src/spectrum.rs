//! Spectrum analysis: a tap on the main track feeds a lock-free ring buffer, and an FFT folds it into bands.

use std::sync::Arc;
use std::sync::atomic::{AtomicU32, AtomicUsize, Ordering};

use kira::Frame;
use kira::effect::{Effect, EffectBuilder};
use realfft::num_complex::Complex;
use realfft::{RealFftPlanner, RealToComplex};

pub const BANDS: usize = 64;
const WINDOW: usize = 2048;
const RING: usize = 1 << 17;
const LOW_HZ: f32 = 30.0;
const HIGH_HZ: f32 = 16000.0;
/// Level range mapped to 0.0..=1.0.
const FLOOR_DB: f32 = -66.0;
const RANGE_DB: f32 = 60.0;

/// Mono ring buffer: the audio thread writes, the analysis side reads.
pub(crate) struct Ring {
    data: Vec<AtomicU32>,
    written: AtomicUsize,
    sample_rate: AtomicU32,
}

impl Ring {
    pub(crate) fn new() -> Arc<Self> {
        Arc::new(Self {
            data: (0..RING).map(|_| AtomicU32::new(0)).collect(),
            written: AtomicUsize::new(0),
            sample_rate: AtomicU32::new(0),
        })
    }
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

pub(crate) struct TapBuilder(pub(crate) Arc<Ring>);

impl EffectBuilder for TapBuilder {
    type Handle = ();

    fn build(self) -> (Box<dyn Effect>, ()) {
        (Box::new(Tap(self.0)), ())
    }
}

pub(crate) struct Analyzer {
    ring: Arc<Ring>,
    read: usize,
    latest: Vec<f32>,
    fft: Arc<dyn RealToComplex<f32>>,
    input: Vec<f32>,
    output: Vec<Complex<f32>>,
    scratch: Vec<Complex<f32>>,
    hann: Vec<f32>,
    bands: [f32; BANDS],
}

impl Analyzer {
    pub(crate) fn new(ring: Arc<Ring>) -> Self {
        let fft = RealFftPlanner::<f32>::new().plan_fft_forward(WINDOW);
        Self {
            read: ring.written.load(Ordering::Acquire),
            ring,
            latest: Vec::with_capacity(WINDOW * 2),
            input: fft.make_input_vec(),
            output: fft.make_output_vec(),
            scratch: fft.make_scratch_vec(),
            fft,
            hann: (0..WINDOW)
                .map(|i| 0.5 - 0.5 * (std::f32::consts::TAU * i as f32 / (WINDOW - 1) as f32).cos())
                .collect(),
            bands: [0.0; BANDS],
        }
    }

    /// Drops everything written so far, so a later `levels` starts from fresh samples.
    pub(crate) fn skip(&mut self) {
        self.read = self.ring.written.load(Ordering::Acquire);
        self.latest.clear();
    }

    /// Band levels from the newest samples, 0.0 to 1.0, low frequencies first. All zero until a full window arrived.
    pub(crate) fn levels(&mut self) -> &[f32; BANDS] {
        let written = self.ring.written.load(Ordering::Acquire);
        // If we fell a full ring behind, the oldest samples are gone.
        self.read = self.read.max(written.saturating_sub(RING));
        self.latest.extend(
            (self.read..written)
                .map(|i| f32::from_bits(self.ring.data[i % RING].load(Ordering::Relaxed))),
        );
        self.read = written;
        if self.latest.len() > WINDOW {
            self.latest.drain(..self.latest.len() - WINDOW);
        }
        if self.latest.len() < WINDOW {
            self.bands.fill(0.0);
            return &self.bands;
        }

        for (slot, (s, w)) in self
            .input
            .iter_mut()
            .zip(self.latest.iter().zip(&self.hann))
        {
            *slot = s * w;
        }
        self.fft
            .process_with_scratch(&mut self.input, &mut self.output, &mut self.scratch)
            .expect("buffers come from the planner");

        let sample_rate = self.ring.sample_rate.load(Ordering::Relaxed) as f32;
        let bin_hz = sample_rate / WINDOW as f32;
        self.bands.fill(0.0);
        for (bin, c) in self.output.iter().enumerate().skip(1) {
            let hz = bin as f32 * bin_hz;
            if !(LOW_HZ..HIGH_HZ).contains(&hz) {
                continue;
            }
            let band = ((hz / LOW_HZ).ln() / (HIGH_HZ / LOW_HZ).ln() * BANDS as f32) as usize;
            let band = band.min(BANDS - 1);
            self.bands[band] = self.bands[band].max(c.norm() / WINDOW as f32);
        }
        for level in &mut self.bands {
            // The Hann window halves a full-scale sine to 0.25 per bin, so scale by 4 to put it at 0 dB.
            *level =
                ((20.0 * (*level * 4.0).max(1e-6).log10() - FLOOR_DB) / RANGE_DB).clamp(0.0, 1.0);
        }
        &self.bands
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn feed(ring: &Ring, samples: impl Iterator<Item = f32>) {
        let mut at = ring.written.load(Ordering::Relaxed);
        for s in samples {
            ring.data[at % RING].store(s.to_bits(), Ordering::Relaxed);
            at += 1;
        }
        ring.written.store(at, Ordering::Release);
    }

    #[test]
    fn a_sine_lights_the_band_of_its_frequency_and_silence_lights_none() {
        let ring = Ring::new();
        ring.sample_rate.store(48000, Ordering::Relaxed);
        let mut analyzer = Analyzer::new(ring.clone());
        assert!(
            analyzer.levels().iter().all(|&l| l == 0.0),
            "not enough samples yet"
        );

        let hz = 1000.0;
        feed(
            &ring,
            (0..WINDOW).map(|i| (std::f32::consts::TAU * hz * i as f32 / 48000.0).sin() * 0.5),
        );
        let levels = *analyzer.levels();
        let loudest = (0..BANDS)
            .max_by(|&a, &b| levels[a].total_cmp(&levels[b]))
            .unwrap();
        let expected = ((hz / LOW_HZ).ln() / (HIGH_HZ / LOW_HZ).ln() * BANDS as f32) as usize;
        assert!(loudest.abs_diff(expected) <= 1, "{loudest} vs {expected}");
        assert!(levels[loudest] > 0.5);

        feed(&ring, std::iter::repeat_n(0.0, WINDOW));
        assert!(analyzer.levels().iter().all(|&l| l == 0.0));
    }

    #[test]
    fn skip_discards_old_samples() {
        let ring = Ring::new();
        ring.sample_rate.store(48000, Ordering::Relaxed);
        let mut analyzer = Analyzer::new(ring.clone());
        feed(&ring, (0..WINDOW).map(|i| (i as f32 * 0.3).sin()));
        analyzer.skip();
        assert!(analyzer.levels().iter().all(|&l| l == 0.0));
    }
}
