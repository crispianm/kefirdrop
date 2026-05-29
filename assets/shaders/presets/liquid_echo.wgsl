fn render(uv: vec2<f32>) -> vec4<f32> {
    let drift = vec2<f32>(sin(uv.y * 15.0 + u.time) * 0.005, cos(uv.x * 15.0 + u.time) * 0.005);
    let suv = uv + drift * (1.0 + u.bass * 2.0);
    var c = sample_prev(suv).rgb * 0.92;
    let line = smoothstep(0.02, 0.0, abs(uv.x - 0.5 + sin(uv.y * 5.0 + u.time * 2.0) * 0.2 * u.mid));
    c += hsv2rgb(vec3<f32>(u.time * 0.2 + uv.y, 0.9, line * u.volume));
    return vec4<f32>(c, 1.0);
}
