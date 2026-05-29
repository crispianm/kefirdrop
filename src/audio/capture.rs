//! Audio capture. Runs on a dedicated thread and feeds mono samples into a
//! lock-free ring buffer drained by the render thread.
//!
//! Two backends:
//! * **pulse** (feature-gated): records from the PipeWire/PulseAudio monitor of
//!   the default sink, so the visualizer reacts to whatever is playing.
//! * **synthetic**: a generated test signal, always available, used when the
//!   `pulse` feature is off or `--synthetic` is passed.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::JoinHandle;

use anyhow::Result;
use ringbuf::traits::{Consumer, Producer, Split};
use ringbuf::{HeapCons, HeapProd, HeapRb};

use super::analysis::Analyzer;
use super::SAMPLE_RATE;

/// Capture configuration handed to [`spawn_audio`].
#[derive(Clone, Debug)]
pub struct AudioConfig {
    pub synthetic: bool,
    /// Only consulted by the `pulse` capture backend.
    #[cfg_attr(not(feature = "pulse"), allow(dead_code))]
    pub device: Option<String>,
    pub sample_rate: u32,
}

impl Default for AudioConfig {
    fn default() -> Self {
        Self {
            synthetic: false,
            device: None,
            sample_rate: SAMPLE_RATE,
        }
    }
}

/// Render-thread side of the audio stream. Owns the ring-buffer consumer and a
/// rolling window of the most recent samples.
pub struct AudioHandle {
    consumer: HeapCons<f32>,
    window: Vec<f32>,
    running: Arc<AtomicBool>,
    _thread: JoinHandle<()>,
}

impl AudioHandle {
    /// Drain everything the capture thread has produced and return the rolling
    /// window of the most recent samples (length == [`Analyzer::window_size`]).
    pub fn samples(&mut self) -> &[f32] {
        let mut chunk = [0.0f32; 4096];
        loop {
            let n = self.consumer.pop_slice(&mut chunk);
            if n == 0 {
                break;
            }
            self.push_window(&chunk[..n]);
        }
        &self.window
    }

    /// Append `new` to the rolling window, discarding the oldest samples.
    fn push_window(&mut self, new: &[f32]) {
        let w = self.window.len();
        if new.len() >= w {
            self.window.copy_from_slice(&new[new.len() - w..]);
        } else {
            self.window.copy_within(new.len().., 0);
            let start = w - new.len();
            self.window[start..].copy_from_slice(new);
        }
    }
}

impl Drop for AudioHandle {
    fn drop(&mut self) {
        self.running.store(false, Ordering::Relaxed);
    }
}

/// Start the capture thread and return a handle for the render thread.
pub fn spawn_audio(cfg: AudioConfig) -> Result<AudioHandle> {
    // ~1 second of headroom; the render thread drains every frame.
    let capacity = cfg.sample_rate as usize;
    let rb = HeapRb::<f32>::new(capacity);
    let (producer, consumer) = rb.split();

    let running = Arc::new(AtomicBool::new(true));
    let thread_running = running.clone();

    // Decide whether to run the synthetic generator. Without the `pulse`
    // feature there is no real backend, so we always fall back to synthetic.
    #[cfg(feature = "pulse")]
    let use_synthetic = cfg.synthetic;
    #[cfg(not(feature = "pulse"))]
    let use_synthetic = {
        if !cfg.synthetic {
            log::warn!("built without the `pulse` feature; using synthetic audio");
        }
        true
    };

    let builder = std::thread::Builder::new().name("kefirdrop-audio".into());
    let handle = if use_synthetic {
        log::info!("audio: synthetic test signal");
        builder.spawn(move || synthetic_loop(producer, thread_running, cfg.sample_rate))?
    } else {
        #[cfg(feature = "pulse")]
        {
            builder.spawn(move || {
                if let Err(e) = pulse_loop(producer, thread_running, &cfg) {
                    log::error!("audio capture failed: {e:#}");
                }
            })?
        }
        #[cfg(not(feature = "pulse"))]
        {
            unreachable!("use_synthetic is always true without the pulse feature")
        }
    };

    Ok(AudioHandle {
        consumer,
        window: vec![0.0; Analyzer::window_size()],
        running,
        _thread: handle,
    })
}

/// Generate a musical-ish test signal: a thumping bass with a swept mid tone and
/// a little noise, paced in real time so analysis behaves like live audio.
fn synthetic_loop(mut producer: HeapProd<f32>, running: Arc<AtomicBool>, rate: u32) {
    use std::f32::consts::TAU;

    const CHUNK: usize = 512;
    let dt = 1.0 / rate as f32;
    let mut t = 0.0f32;
    let mut rng: u32 = 0x1234_5678;
    let mut buf = [0.0f32; CHUNK];

    while running.load(Ordering::Relaxed) {
        for s in buf.iter_mut() {
            // ~120 BPM kick envelope.
            let phase = (t * 2.0).fract();
            let env = (1.0 - phase).powi(6);
            let bass = (t * TAU * 55.0).sin() * 0.6 * env;
            // Slowly swept mid tone.
            let sweep = 440.0 + 250.0 * (t * 0.15 * TAU).sin();
            let mid = (t * TAU * sweep).sin() * 0.18;
            // Cheap xorshift noise for some treble content.
            rng ^= rng << 13;
            rng ^= rng >> 17;
            rng ^= rng << 5;
            let noise = (rng as f32 / u32::MAX as f32 - 0.5) * 0.06;
            *s = (bass + mid + noise).clamp(-1.0, 1.0);
            t += dt;
        }
        producer.push_slice(&buf);
        std::thread::sleep(std::time::Duration::from_secs_f32(CHUNK as f32 * dt));
    }
}

/// Record from the PipeWire/PulseAudio monitor source.
#[cfg(feature = "pulse")]
fn pulse_loop(
    mut producer: HeapProd<f32>,
    running: Arc<AtomicBool>,
    cfg: &AudioConfig,
) -> Result<()> {
    use anyhow::anyhow;
    use libpulse_binding::sample::{Format, Spec};
    use libpulse_binding::stream::Direction;
    use libpulse_simple_binding::Simple;

    let device = resolve_device(cfg);
    match &device {
        Some(d) => log::info!("audio: capturing from `{d}`"),
        None => log::info!("audio: capturing from the default source"),
    }

    let spec = Spec {
        format: Format::F32le,
        channels: 2,
        rate: cfg.sample_rate,
    };
    if !spec.is_valid() {
        return Err(anyhow!("invalid PulseAudio sample spec"));
    }

    let simple = Simple::new(
        None,              // default server
        "kefirdrop",       // application name
        Direction::Record, // capture
        device.as_deref(), // monitor source (or default)
        "system monitor",  // stream description
        &spec,
        None, // default channel map
        None, // default buffering
    )
    .map_err(|e| anyhow!("could not connect to PulseAudio/PipeWire: {e}"))?;

    const FRAMES: usize = 1024;
    let mut frames = [0.0f32; FRAMES * 2]; // interleaved stereo
    let mut mono = [0.0f32; FRAMES];

    while running.load(Ordering::Relaxed) {
        {
            let bytes: &mut [u8] = bytemuck::cast_slice_mut(&mut frames);
            simple
                .read(bytes)
                .map_err(|e| anyhow!("audio read error: {e}"))?;
        }
        for (i, m) in mono.iter_mut().enumerate() {
            *m = 0.5 * (frames[2 * i] + frames[2 * i + 1]);
        }
        // Drop samples if the consumer is behind; visualization only needs the
        // most recent audio.
        producer.push_slice(&mono);
    }
    Ok(())
}

/// Resolve the capture source: explicit config, then env var, then the default
/// sink's monitor (queried with `pactl`).
#[cfg(feature = "pulse")]
fn resolve_device(cfg: &AudioConfig) -> Option<String> {
    if let Some(d) = &cfg.device {
        return Some(d.clone());
    }
    if let Ok(d) = std::env::var("KEFIRDROP_AUDIO_DEVICE") {
        if !d.trim().is_empty() {
            return Some(d.trim().to_string());
        }
    }
    // `pactl get-default-sink` yields e.g. `alsa_output.pci-0000_00_1f.3.analog-stereo`;
    // its monitor source is that name with a `.monitor` suffix.
    match std::process::Command::new("pactl")
        .arg("get-default-sink")
        .output()
    {
        Ok(out) if out.status.success() => {
            let name = String::from_utf8_lossy(&out.stdout).trim().to_string();
            if name.is_empty() {
                None
            } else {
                Some(format!("{name}.monitor"))
            }
        }
        _ => {
            log::warn!("could not run `pactl get-default-sink`; using default source");
            None
        }
    }
}
