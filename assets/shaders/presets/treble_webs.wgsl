fn render(uv: vec2<f32>) -> vec4<f32> {
    let suv = uv + vec2<f32>(sin(u.time * 10.0) * 0.002, cos(u.time * 10.0) * 0.002);
    var c = sample_prev(suv).rgb * 0.88;

    let p = uv - 0.5;
    let a = atan2(p.y, p.x);
    let web = sin(length(p) * 20.0 + a * 10.0 + u.time * 5.0);
    let line = smoothstep(0.95, 1.0, web) * u.treble;

    c += hsv2rgb(vec3<f32>(0.5 + u.time * 0.1, 0.6, line));
    return vec4<f32>(c, 1.0);
}
