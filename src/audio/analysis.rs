//! Turns a window of raw audio samples into perceptual [`Features`] for the
//! shaders: a log-scaled spectrum, band energies, loudness and a beat pulse.

use std::sync::Arc;

use rustfft::{num_complex::Complex, Fft, FftPlanner};

use super::SAMPLE_RATE;

/// Number of spectrum bins exposed to shaders. Matches the WGSL prelude, which
/// packs these into `array<vec4<f32>, NUM_BINS / 4>`, so keep it a multiple of 4.
pub const NUM_BINS: usize = 64;

/// FFT window size. A power of two keeps rustfft fast. ~46 ms at 44.1 kHz,
/// a good balance between frequency and time resolution for music.
const FFT_SIZE: usize = 2048;

/// Perceptual features derived from the audio, consumed by the renderer.
#[derive(Clone, Debug)]
pub struct Features {
    /// Normalized, smoothed spectrum magnitudes in `[0, 1]`, log-spaced in
    /// frequency so bass occupies a fair share of the bins.
    pub spectrum: [f32; NUM_BINS],
    /// Smoothed low-frequency energy in `[0, 1]`.
    pub bass: f32,
    /// Smoothed mid-frequency energy in `[0, 1]`.
    pub mid: f32,
    /// Smoothed high-frequency energy in `[0, 1]`.
    pub treble: f32,
    /// Smoothed RMS loudness in `[0, 1]`.
    pub volume: f32,
    /// Beat pulse in `[0, 1]`. Jumps to 1.0 on a detected onset and decays.
    pub beat: f32,
}

impl Default for Features {
    fn default() -> Self {
        Self {
            spectrum: [0.0; NUM_BINS],
            bass: 0.0,
            mid: 0.0,
            treble: 0.0,
            volume: 0.0,
            beat: 0.0,
        }
    }
}

pub struct Analyzer {
    fft: Arc<dyn Fft<f32>>,
    window: Vec<f32>,
    scratch: Vec<Complex<f32>>,
    /// Maps each output bin to the inclusive FFT-bin range it averages over.
    bin_ranges: Vec<(usize, usize)>,
    features: Features,
    /// Running average of bass energy for onset detection.
    bass_avg: f32,
}

impl Analyzer {
    pub fn new() -> Self {
        let mut planner = FftPlanner::new();
        let fft = planner.plan_fft_forward(FFT_SIZE);

        // Hann window reduces spectral leakage.
        let window = (0..FFT_SIZE)
            .map(|n| {
                let x = std::f32::consts::PI * n as f32 / (FFT_SIZE as f32 - 1.0);
                x.sin().powi(2)
            })
            .collect();

        // Pre-compute log-spaced bin boundaries across the usable spectrum
        // (skip DC, stop at Nyquist).
        let usable = FFT_SIZE / 2;
        let min_bin = 1.0f32;
        let max_bin = usable as f32;
        let bin_ranges = (0..NUM_BINS)
            .map(|i| {
                let lo = min_bin * (max_bin / min_bin).powf(i as f32 / NUM_BINS as f32);
                let hi = min_bin * (max_bin / min_bin).powf((i + 1) as f32 / NUM_BINS as f32);
                let lo = lo.floor() as usize;
                let hi = (hi.ceil() as usize).max(lo + 1).min(usable);
                (lo, hi)
            })
            .collect();

        Self {
            fft,
            window,
            scratch: vec![Complex::new(0.0, 0.0); FFT_SIZE],
            bin_ranges,
            features: Features::default(),
            bass_avg: 0.0,
        }
    }

    /// Number of samples the analyzer wants per call.
    pub const fn window_size() -> usize {
        FFT_SIZE
    }

    /// Run analysis over the most recent `FFT_SIZE` samples. `samples` may be
    /// shorter (start-up); missing samples are treated as silence. `dt` is the
    /// time since the previous call, used for frame-rate-independent smoothing.
    pub fn process(&mut self, samples: &[f32], dt: f32) -> &Features {
        // Fill the FFT input, windowed, newest-aligned to the end.
        let n = samples.len().min(FFT_SIZE);
        let pad = FFT_SIZE - n;
        for c in self.scratch.iter_mut().take(pad) {
            *c = Complex::new(0.0, 0.0);
        }
        let src = &samples[samples.len() - n..];
        for (i, &sample) in src.iter().enumerate() {
            let w = self.window[pad + i];
            self.scratch[pad + i] = Complex::new(sample * w, 0.0);
        }

        self.fft.process(&mut self.scratch);

        // Magnitudes, grouped into log-spaced bins. Normalize by FFT size and
        // apply a perceptual-ish log compression.
        let norm = 2.0 / FFT_SIZE as f32;
        let mut raw = [0.0f32; NUM_BINS];
        for (out, &(lo, hi)) in raw.iter_mut().zip(&self.bin_ranges) {
            let mut acc = 0.0;
            for k in lo..hi {
                acc += self.scratch[k].norm() * norm;
            }
            let avg = acc / (hi - lo) as f32;
            // Log compression maps a wide dynamic range into [0, ~1].
            *out = (1.0 + avg * 32.0).ln() / 4.0;
        }

        // Temporal smoothing: fast attack, slow release feels musical.
        let attack = 1.0 - (-dt / 0.02).exp();
        let release = 1.0 - (-dt / 0.18).exp();
        for (slot, &raw_mag) in self.features.spectrum.iter_mut().zip(raw.iter()) {
            let prev = *slot;
            let target = raw_mag.clamp(0.0, 1.0);
            let rate = if target > prev { attack } else { release };
            *slot = prev + (target - prev) * rate;
        }

        // Band energies from frequency cut-offs.
        let hz_to_bin = |hz: f32| (hz * FFT_SIZE as f32 / SAMPLE_RATE as f32) as usize;
        let band = |lo_hz: f32, hi_hz: f32| -> f32 {
            let lo = hz_to_bin(lo_hz).max(1);
            let hi = hz_to_bin(hi_hz).min(FFT_SIZE / 2).max(lo + 1);
            let mut acc = 0.0;
            for k in lo..hi {
                acc += self.scratch[k].norm() * norm;
            }
            let avg = acc / (hi - lo) as f32;
            ((1.0 + avg * 32.0).ln() / 4.0).clamp(0.0, 1.0)
        };
        let bass = band(20.0, 150.0);
        let mid = band(150.0, 2_000.0);
        let treble = band(2_000.0, 16_000.0);

        let smooth = |prev: f32, target: f32| {
            let rate = if target > prev { attack } else { release };
            prev + (target - prev) * rate
        };
        self.features.bass = smooth(self.features.bass, bass);
        self.features.mid = smooth(self.features.mid, mid);
        self.features.treble = smooth(self.features.treble, treble);

        // RMS loudness over the raw (un-windowed) tail.
        let rms = if n > 0 {
            (src.iter().map(|s| s * s).sum::<f32>() / n as f32).sqrt()
        } else {
            0.0
        };
        self.features.volume = smooth(self.features.volume, (rms * 4.0).clamp(0.0, 1.0));

        // Beat detection: onset when instantaneous bass clearly exceeds its
        // running average. The pulse decays exponentially between hits.
        let beat_decay = 1.0 - (-dt / 0.12).exp();
        self.features.beat = (self.features.beat - self.features.beat * beat_decay).max(0.0);
        if bass > self.bass_avg * 1.4 && bass > 0.1 {
            self.features.beat = 1.0;
        }
        self.bass_avg = self.bass_avg * 0.92 + bass * 0.08;

        &self.features
    }
}

impl Default for Analyzer {
    fn default() -> Self {
        Self::new()
    }
}
