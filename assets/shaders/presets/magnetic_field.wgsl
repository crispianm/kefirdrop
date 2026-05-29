fn render(uv: vec2<f32>) -> vec4<f32> {
    let p = (uv - 0.5) * 2.0;
    let r2 = dot(p, p);
    let field = (3.0 * p.x * p.y) / (r2 * r2 + 0.1);
    let lines = smoothstep(0.9, 1.0, sin(field * 10.0 - u.time * 2.0));

    let suv = (uv - 0.5) * 0.98 + 0.5;
    var c = sample_prev(suv).rgb * 0.93;

    c += hsv2rgb(vec3<f32>(field * 0.1 + u.time * 0.1, 0.8, lines * u.volume));
    return vec4<f32>(c, 1.0);
}
