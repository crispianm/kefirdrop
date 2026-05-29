fn render(uv: vec2<f32>) -> vec4<f32> {
    let p = uv - 0.5;
    let r = length(p);
    let a = atan2(p.y, p.x);

    let rays = smoothstep(0.9, 1.0, sin(a * 20.0 + u.time * 5.0));
    let nova = smoothstep(0.3 + u.beat * 0.2, 0.0, r) * rays * u.beat;

    let suv = p * (1.0 - u.treble * 0.05) + 0.5;
    var c = sample_prev(suv).rgb * 0.9;
    c += hsv2rgb(vec3<f32>(u.time * 0.5, 0.7, nova));
    return vec4<f32>(c, 1.0);
}
