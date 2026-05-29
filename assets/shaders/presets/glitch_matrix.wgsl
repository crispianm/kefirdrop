fn render(uv: vec2<f32>) -> vec4<f32> {
    let grid = 50.0 - u.bass * 20.0;
    let q_uv = floor(uv * grid) / grid;
    let zoom = 1.05;
    let suv = (q_uv - 0.5) / zoom + 0.5;

    var c = sample_prev(suv).rgb * 0.85;
    let scanline = smoothstep(0.8, 1.0, sin(uv.y * 100.0 + u.time * 10.0));
    c += hsv2rgb(vec3<f32>(0.3, 1.0, scanline * u.volume * 0.2));

    let bin = i32(q_uv.x * 64.0);
    let mag = spectrum_at(bin);
    let bar = smoothstep(0.01, 0.0, abs((1.0 - q_uv.y) - mag));
    c += hsv2rgb(vec3<f32>(q_uv.x, 0.8, bar));

    return vec4<f32>(c, 1.0);
}
