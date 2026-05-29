fn render(uv: vec2<f32>) -> vec4<f32> {
    var p = uv - 0.5;
    p = abs(p) - 0.1 * u.bass;
    let suv = p * 1.05 + 0.5;
    var c = sample_prev(suv).rgb * 0.95;

    let center = smoothstep(0.1, 0.0, length(uv - 0.5));
    c += hsv2rgb(vec3<f32>(u.time * 0.1, 0.8, center * u.beat));
    return vec4<f32>(c, 1.0);
}
