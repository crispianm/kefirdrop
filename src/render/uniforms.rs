//! GPU uniform block shared with the shaders.
//!
//! The byte layout here must match the `Uniforms` struct in
//! `assets/shaders/prelude.wgsl` exactly. Field order and the trailing
//! `spectrum` array are arranged so the struct needs no padding and the
//! `vec4` array lands on a 16-byte boundary (offset 32).

use crate::audio::{Features, NUM_BINS};

/// Number of `vec4`s used to pack the spectrum (`NUM_BINS` floats, 4 per vec4).
pub const SPECTRUM_VEC4S: usize = NUM_BINS / 4;

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Uniforms {
    /// Render target size in pixels.
    pub resolution: [f32; 2],
    /// Seconds since launch.
    pub time: f32,
    /// Beat pulse in `[0, 1]`.
    pub beat: f32,
    /// Low-band energy in `[0, 1]`.
    pub bass: f32,
    /// Mid-band energy in `[0, 1]`.
    pub mid: f32,
    /// High-band energy in `[0, 1]`.
    pub treble: f32,
    /// RMS loudness in `[0, 1]`.
    pub volume: f32,
    /// Log-scaled spectrum, packed 4 bins per `vec4`.
    pub spectrum: [[f32; 4]; SPECTRUM_VEC4S],
}

impl Default for Uniforms {
    fn default() -> Self {
        Self {
            resolution: [1.0, 1.0],
            time: 0.0,
            beat: 0.0,
            bass: 0.0,
            mid: 0.0,
            treble: 0.0,
            volume: 0.0,
            spectrum: [[0.0; 4]; SPECTRUM_VEC4S],
        }
    }
}

impl Uniforms {
    /// Refresh the per-frame values from analysis output.
    pub fn update(&mut self, resolution: [f32; 2], time: f32, features: &Features) {
        self.resolution = resolution;
        self.time = time;
        self.beat = features.beat;
        self.bass = features.bass;
        self.mid = features.mid;
        self.treble = features.treble;
        self.volume = features.volume;
        for (i, &v) in features.spectrum.iter().enumerate() {
            self.spectrum[i / 4][i % 4] = v;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layout_matches_wgsl() {
        // 8 scalars (2 + 6) before the spectrum array, then NUM_BINS floats.
        // The array must start at a 16-byte boundary (offset 32) and the whole
        // block must be 16-byte aligned, matching `Uniforms` in prelude.wgsl.
        let scalar_bytes = (2 + 6) * std::mem::size_of::<f32>();
        assert_eq!(scalar_bytes, 32, "spectrum must begin at offset 32");
        let expected = scalar_bytes + NUM_BINS * std::mem::size_of::<f32>();
        assert_eq!(std::mem::size_of::<Uniforms>(), expected);
        assert_eq!(std::mem::size_of::<Uniforms>() % 16, 0);
    }
}
