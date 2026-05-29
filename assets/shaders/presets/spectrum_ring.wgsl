fn render(uv: vec2<f32>) -> vec4<f32> {
    let p = uv - 0.5;
    let r = length(p);
    let a = atan2(p.y, p.x);
    let normalized_a = (a + 3.14159) / 6.28318;
    let bin = i32(normalized_a * 64.0);
    let mag = spectrum_at(bin);

    let target_r = 0.2 + mag * 0.3;
    let ring = smoothstep(0.02, 0.0, abs(r - target_r));
    let suv = p * 1.02 + 0.5;
    var c = sample_prev(suv).rgb * 0.85;

    c += hsv2rgb(vec3<f32>(normalized_a + u.time * 0.1, 0.8, ring));
    return vec4<f32>(c, 1.0);
}
