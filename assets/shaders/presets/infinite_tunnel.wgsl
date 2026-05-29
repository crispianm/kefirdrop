fn render(uv: vec2<f32>) -> vec4<f32> {
    let p = uv - 0.5;
    let max_xy = max(abs(p.x), abs(p.y));
    let zoom = 0.95 - u.bass * 0.05;
    let suv = p * zoom + 0.5;

    var c = sample_prev(suv).rgb * 0.96;
    let square = smoothstep(0.02, 0.0, abs(max_xy - 0.4));
    c += hsv2rgb(vec3<f32>(u.time * 0.2, 0.9, square * u.beat));
    return vec4<f32>(c, 1.0);
}
