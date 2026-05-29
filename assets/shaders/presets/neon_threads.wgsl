fn render(uv: vec2<f32>) -> vec4<f32> {
    let suv = uv + vec2<f32>(0.005, 0.0);
    var c = sample_prev(suv).rgb * 0.94;

    let wave = sin(uv.y * 10.0 + u.time * 5.0) * 0.1 * u.mid;
    let thread = smoothstep(0.01, 0.0, abs(uv.x - 0.2 - wave));

    c += hsv2rgb(vec3<f32>(uv.y + u.time * 0.1, 0.8, thread));
    return vec4<f32>(c, 1.0);
}
