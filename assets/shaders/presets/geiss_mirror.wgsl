fn render(uv: vec2<f32>) -> vec4<f32> {
    var p = uv - 0.5;
    p = vec2<f32>(abs(p.x), abs(p.y));
    let rot = u.time * 0.1;
    let ca = cos(rot);
    let sa = sin(rot);
    p = vec2<f32>(p.x * ca - p.y * sa, p.x * sa + p.y * ca);

    let suv = p * 0.98 + 0.5;
    var c = sample_prev(suv).rgb * 0.94;

    let dot = smoothstep(0.02, 0.0, length(uv - vec2<f32>(0.5, 0.2 + u.bass * 0.2)));
    c += hsv2rgb(vec3<f32>(u.time * 0.2, 0.9, dot));
    return vec4<f32>(c, 1.0);
}
