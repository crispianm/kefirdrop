fn render(uv: vec2<f32>) -> vec4<f32> {
    let p = uv - 0.5;
    let r = length(p);
    let a = atan2(p.y, p.x);
    let twist = sin(r * 10.0 - u.time * 2.0) * 0.1 * u.mid;
    let angle = a + twist;
    let suv = vec2<f32>(cos(angle), sin(angle)) * r * 0.98 + 0.5;

    var c = sample_prev(suv).rgb * 0.96;
    let petals = sin(a * 8.0 + u.time) * 0.05;
    let shape = smoothstep(0.02, 0.0, abs(r - 0.3 - petals - u.bass * 0.1));
    c += hsv2rgb(vec3<f32>(r * 3.0 - u.time, 0.9, shape * u.volume));
    return vec4<f32>(c, 1.0);
}
