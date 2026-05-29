fn render(uv: vec2<f32>) -> vec4<f32> {
    var p = uv - 0.5;
    let a = atan2(p.y, p.x);
    let r = length(p);

    let segments = 3.0;
    let folded_a = abs(fract(a / 6.28318 * segments) - 0.5) * 6.28318 / segments;
    p = vec2<f32>(cos(folded_a), sin(folded_a)) * r;

    let suv = p * 0.95 + 0.5;
    var c = sample_prev(suv).rgb * 0.92;

    let line = smoothstep(0.02, 0.0, abs(p.x - 0.2 - u.treble * 0.1));
    c += hsv2rgb(vec3<f32>(r * 2.0 + u.time, 0.8, line));
    return vec4<f32>(c, 1.0);
}
