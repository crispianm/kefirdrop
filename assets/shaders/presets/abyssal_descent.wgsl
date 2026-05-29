fn render(uv: vec2<f32>) -> vec4<f32> {
    let p = uv - 0.5;
    let r = length(p);
    let zoom = 1.05 + u.bass * 0.1;
    let suv = p * zoom + 0.5;

    var c = sample_prev(suv).rgb * 0.95;
    c *= smoothstep(0.0, 0.1, r);

    let edge = smoothstep(0.4, 0.5, r) * smoothstep(0.6, 0.5, r);
    c += hsv2rgb(vec3<f32>(u.time * 0.05, 1.0, edge * u.treble));
    return vec4<f32>(c, 1.0);
}
