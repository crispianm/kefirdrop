# Presets

A preset is two files:

1. A **manifest** — `assets/presets/<name>.toml`
2. A **WGSL body** — referenced by the manifest, e.g.
   `assets/shaders/presets/<name>.wgsl`

## Manifest format

```toml
name = "Plasma"                          # shown in logs, used by --preset
shader = "shaders/presets/plasma.wgsl"   # path relative to the assets dir
description = "Layered sine plasma."     # optional
```

Presets are discovered by scanning `assets/presets/*.toml` and sorted by
`name`, which is also the cycling order (`→` / `←`).

## Writing the shader

A preset body defines exactly one function:

```wgsl
fn render(uv: vec2<f32>) -> vec4<f32> {
    // uv is in [0, 1], origin top-left
    return vec4<f32>(uv, 0.0, 1.0);
}
```

The prelude (`assets/shaders/prelude.wgsl`) is prepended automatically, so the
following are available without declaring them:

### Uniforms (`u`)

| Field           | Meaning                                   |
| --------------- | ----------------------------------------- |
| `u.resolution`  | render size in pixels (`vec2<f32>`)       |
| `u.time`        | seconds since launch                      |
| `u.beat`        | beat pulse, `0..1`, decays between hits   |
| `u.bass`        | low-band energy, `0..1`                   |
| `u.mid`         | mid-band energy, `0..1`                   |
| `u.treble`      | high-band energy, `0..1`                  |
| `u.volume`      | RMS loudness, `0..1`                      |
| `u.spectrum`    | 64 log bins packed as `array<vec4<f32>,16>` |

### Helpers

- `spectrum_at(i: i32) -> f32` — spectrum bin `i` (0–63), clamped.
- `sample_prev(uv: vec2<f32>) -> vec4<f32>` — sample the previous frame for
  feedback. Sampling at a transformed `uv` (zoom/rotate) is how you get
  trails and warps.
- `hsv2rgb(c: vec3<f32>) -> vec3<f32>` — HSV (components `0..1`) to RGB.

Output is **linear** colour; the final blit handles sRGB encoding. The
offscreen target is `rgba16float`, so values may exceed 1.0 for glow.

## Iterating

The active preset's `.wgsl` is watched for changes — save the file and it
reloads live. A shader with a compile error logs the error and keeps the last
working version, so a typo won't crash the app. Press `R` to force a reload.

## Validation

`cargo test --no-default-features` parses and validates every preset (prelude +
body) with `naga`, the same WGSL front-end wgpu uses — no GPU required. Add new
presets and the test covers them automatically.

## Future: `.milk` conversion

This format is intentionally the target for a future MilkDrop `.milk` →
WGSL converter: a `.milk` preset's per-frame/per-pixel equations and warp/comp
shaders would be translated into a generated `render()` body plus a manifest,
loaded by the exact same path as hand-written presets.
