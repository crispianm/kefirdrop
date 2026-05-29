// A feedback tunnel: each frame samples a zoomed + rotated copy of the previous
// frame and injects fresh colour at the centre. Bass drives the zoom, treble the
// rotation — the classic MilkDrop "warp" look.

fn render(uv: vec2<f32>) -> vec4<f32> {
    let aspect = u.resolution.x / max(u.resolution.y, 1.0);
    let p = (uv - vec2<f32>(0.5)) * vec2<f32>(aspect, 1.0);
    let r = length(p);
    let a = atan2(p.y, p.x);

    // Zoom and rotate the previous frame.
    let zoom = 1.02 + u.bass * 0.06;
    let rot = u.time * 0.05 + u.treble * 0.04;
    let ca = cos(rot);
    let sa = sin(rot);
    let q = vec2<f32>(p.x * ca - p.y * sa, p.x * sa + p.y * ca) / zoom;
    let suv = q / vec2<f32>(aspect, 1.0) + vec2<f32>(0.5);

    var c = sample_prev(suv).rgb * 0.96;

    // Inject a coloured, rotating burst near the centre, pumped by the audio.
    let ring = sin(a * 6.0 + u.time * 2.0) * 0.5 + 0.5;
    let hue = fract(u.time * 0.05 + r * 0.5 + ring * 0.1);
    let intensity = smoothstep(0.5, 0.0, r) * (0.08 + u.volume * 0.5 + u.beat * 0.45);
    c += hsv2rgb(vec3<f32>(hue, 0.9, 1.0)) * intensity;

    return vec4<f32>(c, 1.0);
}
