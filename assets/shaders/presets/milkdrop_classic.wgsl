fn render(uv: vec2<f32>) -> vec4<f32> {
    let p = uv - 0.5;
    let r = length(p);
    let a = atan2(p.y, p.x);

    let zoom = 0.98 - u.bass * 0.04;
    let rot = u.time * 0.05 + u.mid * 0.02;
    let ca = cos(rot);
    let sa = sin(rot);
    let q = vec2<f32>(p.x * ca - p.y * sa, p.x * sa + p.y * ca);

    let suv = q * zoom + 0.5;
    var c = sample_prev(suv).rgb * 0.96;

    let star = smoothstep(0.05 + u.beat * 0.1, 0.0, r);
    c += hsv2rgb(vec3<f32>(a * 0.5 + u.time * 0.2, 0.9, star * u.volume));
    return vec4<f32>(c, 1.0);
}
