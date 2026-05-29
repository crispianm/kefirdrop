# Architecture

kefirdrop is a small real-time pipeline: capture audio → analyse it → drive
GPU shaders. Two threads, no async runtime.

```
 ┌──────────────┐   f32 mono samples    ┌───────────────────────────────┐
 │ capture thread│ ───── ring buffer ──▶ │ main / render thread          │
 │ (PipeWire or  │   (lock-free SPSC)    │  Analyzer ─▶ Uniforms ─▶ wgpu  │
 │  synthetic)   │                       │  winit event loop @ vsync     │
 └──────────────┘                        └───────────────────────────────┘
```

## Modules (`src/`)

| Module            | Responsibility |
| ----------------- | -------------- |
| `main.rs`         | Logging, CLI parsing, starts the winit event loop. |
| `config.rs`       | `Config` parsed from CLI args (`--synthetic`, `--device`, …). |
| `audio/capture.rs`| Capture thread + ring buffer. `pulse` backend (PipeWire/Pulse monitor) and a synthetic generator. |
| `audio/analysis.rs`| `Analyzer`: Hann window → FFT → 64 log bins, band energies, loudness, beat. |
| `render/uniforms.rs`| `Uniforms` — the GPU uniform block (must match `prelude.wgsl`). |
| `render/shader.rs`| Assembles the prelude + a preset's `render()` + the entry `fs_main`. |
| `render/renderer.rs`| All wgpu state: pipelines, ping-pong feedback targets, draw. |
| `preset/mod.rs`   | Discovers/loads TOML preset manifests. |
| `app.rs`          | Ties it together: window, surface, frame loop, input, hot-reload. |

## Audio

- **Capture** runs on its own thread so audio I/O never blocks rendering.
  Samples are mixed to mono and pushed into a `ringbuf` SPSC buffer (~1s
  capacity). If the renderer falls behind, the oldest samples are simply
  dropped — only the most recent audio matters for visuals.
- The **pulse** backend resolves the capture device as
  `"$(pactl get-default-sink).monitor"` (overridable via `--device` or
  `KEFIRDROP_AUDIO_DEVICE`) and records via the PulseAudio simple API, which
  PipeWire implements through `pipewire-pulse`.
- **Analysis** (`Analyzer::process`) runs once per frame on the most recent
  2048 samples: Hann window, real FFT (`rustfft`), magnitudes grouped into 64
  log-spaced bins, per-band energies (bass/mid/treble), RMS loudness, and an
  energy-based beat detector. Everything is smoothed with a frame-rate
  independent fast-attack / slow-release filter, then exposed as `Features`.

## Rendering

The renderer is deliberately shader-centric. Each frame:

1. **Scene pass** — draw a fullscreen triangle running the active preset into
   offscreen texture `targets[w]` (`rgba16float`), while binding the *other*
   texture `targets[r]` as `prev_tex`. The preset reads it with `sample_prev`,
   which is what produces feedback trails/warps. The pass uses `REPLACE`
   blending and fully overwrites its target.
2. **Blit pass** — copy `targets[w]` to the swapchain (sRGB encode on store).
3. **Swap** `w`/`r`. Next frame's "previous" is this frame's output.

Two scene bind groups and two blit bind groups are created up front (one per
texture) so swapping is just an index flip — no per-frame allocation. Targets
are recreated on resize and cleared to black so early frames sample defined
data.

The uniform block is uploaded once per frame with `queue.write_buffer`. Its
byte layout (`src/render/uniforms.rs`) is mirrored exactly by the `Uniforms`
struct in `assets/shaders/prelude.wgsl`; `layout_matches_wgsl` guards it.

## Shaders

Presets stay tiny: they only define `fn render(uv) -> vec4<f32>`. At load time
`shader::assemble` concatenates `prelude.wgsl` (uniforms, bindings, vertex
shader, helpers) + the preset body + a fixed `fs_main` that calls `render`.
WGSL has no forward references, so the order matters — `render` is defined
before `fs_main`.

Shaders are validated in CI without a GPU using `naga` (the same front-end
wgpu uses) — see the tests in `render/shader.rs`.

## Build features

- `pulse` (default): real audio capture via `libpulse`.
- Without it (`--no-default-features`): synthetic audio only, so the engine
  builds and tests on machines lacking `libpulse` headers and a GPU.

## Roadmap

- MilkDrop `.milk` → WGSL converter that emits into this preset format (the
  indirection in `preset/` exists for exactly this).
- More presets; per-preset parameters in the manifest.
- Optional on-screen UI / preset browser.
