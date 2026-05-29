fn render(uv: vec2<f32>) -> vec4<f32> {
    let p = uv - 0.5;
    let a = atan2(p.y, p.x);
    let r = length(p);

    let petals = 0.2 + 0.15 * sin(a * 5.0 + u.time);
    let blossom = smoothstep(0.03, 0.0, abs(r - petals - u.bass * 0.2));

    let zoom = 1.01 + u.bass * 0.02;
    let suv = p / zoom + 0.5;
    var c = sample_prev(suv).rgb * 0.93;

    c += hsv2rgb(vec3<f32>(u.time * 0.1 + a * 0.1, 0.8, blossom));
    return vec4<f32>(c, 1.0);
}
