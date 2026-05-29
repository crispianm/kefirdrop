// kefirdrop shader prelude.
//
// This is prepended to every preset. A preset only needs to define:
//
//     fn render(uv: vec2<f32>) -> vec4<f32> { ... }
//
// where `uv` is in [0, 1] with the origin at the top-left. Everything below —
// the audio uniforms, the previous-frame texture and a handful of helpers — is
// available to it. (The matching Rust layout lives in src/render/uniforms.rs.)

struct Uniforms {
    resolution: vec2<f32>,  // render size in pixels
    time: f32,              // seconds since launch
    beat: f32,              // beat pulse, 0..1, decays between hits
    bass: f32,              // low-band energy, 0..1
    mid: f32,               // mid-band energy, 0..1
    treble: f32,            // high-band energy, 0..1
    volume: f32,            // RMS loudness, 0..1
    spectrum: array<vec4<f32>, 16>, // 64 log-scaled bins, 4 per vec4
};

@group(0) @binding(0) var<uniform> u: Uniforms;

// The previous rendered frame, for MilkDrop-style feedback.
@group(1) @binding(0) var prev_tex: texture_2d<f32>;
@group(1) @binding(1) var prev_samp: sampler;

struct VsOut {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

// Fullscreen triangle — no vertex buffer needed.
@vertex
fn vs_main(@builtin(vertex_index) vi: u32) -> VsOut {
    var positions = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>( 3.0, -1.0),
        vec2<f32>(-1.0,  3.0),
    );
    let p = positions[vi];
    var out: VsOut;
    out.position = vec4<f32>(p, 0.0, 1.0);
    // Flip Y so uv (0,0) is the top-left and feedback is orientation-stable.
    out.uv = vec2<f32>(p.x * 0.5 + 0.5, 1.0 - (p.y * 0.5 + 0.5));
    return out;
}

// ---- helpers available to presets ----

// Sample the previous frame at `uv` (same screen location => stable feedback).
fn sample_prev(uv: vec2<f32>) -> vec4<f32> {
    return textureSample(prev_tex, prev_samp, uv);
}

// Read spectrum bin `i` (0..63), clamped.
fn spectrum_at(i: i32) -> f32 {
    let idx = u32(clamp(i, 0, 63));
    return u.spectrum[idx / 4u][idx % 4u];
}

// HSV (all components 0..1) to linear-ish RGB.
fn hsv2rgb(c: vec3<f32>) -> vec3<f32> {
    let k = vec4<f32>(1.0, 2.0 / 3.0, 1.0 / 3.0, 3.0);
    let p = abs(fract(vec3<f32>(c.x) + k.xyz) * 6.0 - vec3<f32>(k.w));
    return c.z * mix(vec3<f32>(k.x), clamp(p - vec3<f32>(k.x), vec3<f32>(0.0), vec3<f32>(1.0)), c.y);
}
