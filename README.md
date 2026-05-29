# kefirdrop

A GPU-accelerated, real-time music visualizer for Linux audiophiles — inspired
by [MilkDrop](https://github.com/milkdrop2077/MilkDrop3), built in Rust on
`wgpu` (Vulkan), and aimed at modern Arch/Wayland desktops.

kefirdrop listens to whatever your system is playing (Spotify, a browser,
anything) via the PipeWire monitor source, runs an FFT over it, and feeds the
result into WGSL shaders with MilkDrop-style frame-feedback for fluid,
audio-reactive visuals.

> Status: early. The render + audio engine is in place with three built-in
> presets. GPU/audio runtime testing happens on real hardware (1080ti and
> newer). See `docs/ARCHITECTURE.md`.

## Features

- **GPU rendering** via `wgpu` → Vulkan (works on a GTX 1080ti and newer).
- **Frame feedback** — ping-pong HDR (`rgba16float`) buffers give the classic
  MilkDrop trails/warps.
- **System-wide audio** — captures the default sink's monitor through
  PipeWire/PulseAudio, so it reacts to any app.
- **Live spectrum analysis** — 64 log-scaled bands, bass/mid/treble energy,
  loudness and beat detection.
- **Hot-reloadable WGSL presets** — edit a shader and see it update live.
- **Synthetic mode** — develop without a sound card or on GPU-less CI.

## Prerequisites (Arch)

```sh
sudo pacman -S rust pipewire pipewire-pulse wireplumber vulkan-icd-loader
# NVIDIA users also need the proprietary driver + Vulkan ICD:
sudo pacman -S nvidia nvidia-utils
```

`pactl` (from `libpulse`) is used to auto-detect the monitor source; it ships
with `pipewire-pulse`.

## Build & run

```sh
cargo run --release
```

Play some audio and the window should react. Useful flags:

```sh
cargo run --release -- --synthetic        # built-in test signal, no real audio
cargo run --release -- --no-vsync         # uncapped frame rate
cargo run --release -- --preset "Plasma"  # start on a named preset
cargo run --release -- --device <SOURCE>  # capture a specific source
cargo run --release -- --help
```

If auto-detection picks the wrong input, set the source explicitly:

```sh
export KEFIRDROP_AUDIO_DEVICE="$(pactl get-default-sink).monitor"
```

List sources with `pactl list sources short`.

### Keys

| Key            | Action                |
| -------------- | --------------------- |
| `→` / `N`      | Next preset           |
| `←` / `P`      | Previous preset       |
| `R`            | Reload current shader |
| `Esc` / `Q`    | Quit                  |

## Building without real audio

CI and headless machines can build the engine without `libpulse`:

```sh
cargo build --no-default-features   # synthetic audio only
cargo test  --no-default-features   # validates shaders + uniform layout
```

## Writing presets

A preset is a tiny TOML manifest in `assets/presets/` pointing at a WGSL file
that defines a single `render(uv)` function. See `docs/PRESETS.md`.

## License

MIT OR Apache-2.0.
