fn render(uv: vec2<f32>) -> vec4<f32> {
    let p1 = uv - vec2<f32>(0.3, 0.5);
    let p2 = uv - vec2<f32>(0.7, 0.5);
    let d1 = length(p1);
    let d2 = length(p2);

    let wave1 = sin(d1 * 40.0 - u.time * 5.0);
    let wave2 = sin(d2 * 40.0 - u.time * 5.0);
    let interference = smoothstep(0.8, 1.0, wave1 * wave2);

    let suv = (uv - 0.5) * 0.99 + 0.5;
    var c = sample_prev(suv).rgb * 0.9;
    c += hsv2rgb(vec3<f32>(u.time * 0.2, 0.8, interference * u.bass));
    return vec4<f32>(c, 1.0);
}
