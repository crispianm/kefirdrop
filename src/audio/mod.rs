//! Audio capture and analysis.
//!
//! The capture side runs on its own thread and pushes interleaved-mixed mono
//! samples into a lock-free ring buffer. The main (render) thread drains the
//! ring buffer once per frame and runs [`analysis::Analyzer`] over it to derive
//! the [`Features`] that drive the shaders.

pub mod analysis;
pub mod capture;

pub use analysis::{Analyzer, Features, NUM_BINS};
pub use capture::{spawn_audio, AudioConfig, AudioHandle};

/// Sample rate we request from the audio server. 44.1 kHz is plenty for
/// visualization and keeps FFT sizes sane.
pub const SAMPLE_RATE: u32 = 44_100;
