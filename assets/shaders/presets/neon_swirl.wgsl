fn render(uv: vec2<f32>) -> vec4<f32> {
    let p = uv - 0.5;
    let r = length(p);
    let a = atan2(p.y, p.x);
    let zoom = 0.98 - u.bass * 0.05;
    let angle = a + 0.05 + u.mid * 0.1;
    let suv = vec2<f32>(cos(angle), sin(angle)) * r * zoom + 0.5;
    var c = sample_prev(suv).rgb * 0.95;
    let ring = smoothstep(0.05, 0.0, abs(r - 0.2 - u.bass * 0.2));
    c += hsv2rgb(vec3<f32>(u.time * 0.1, 0.8, ring * (0.5 + u.beat)));
    return vec4<f32>(c, 1.0);
}
