fn render(uv: vec2<f32>) -> vec4<f32> {
    var p = uv - 0.5;
    let r = length(p);
    let a = atan2(p.y, p.x);
    let segments = 6.0;
    let folded_a = abs(fract(a / 6.28318 * segments) - 0.5) * 6.28318 / segments;
    p = vec2<f32>(cos(folded_a), sin(folded_a)) * r;
    let suv = p * 0.98 + 0.5;
    var c = sample_prev(suv).rgb * 0.97;
    let dot = smoothstep(0.05, 0.0, length(p - vec2<f32>(0.1 + u.bass * 0.1, 0.0)));
    c += hsv2rgb(vec3<f32>(r * 2.0 - u.time, 0.8, dot * u.beat));
    return vec4<f32>(max(c, vec3<f32>(0.0)), 1.0);
}
