fn render(uv: vec2<f32>) -> vec4<f32> {
    let p = uv - vec2<f32>(0.5, 0.5);
    let depth = max(abs(p.y), 0.001);
    let x = p.x / depth;
    let z = 1.0 / depth + u.time * 2.0;

    let grid_x = smoothstep(0.05, 0.0, abs(fract(x * 5.0) - 0.5));
    let grid_z = smoothstep(0.05, 0.0, abs(fract(z * 2.0) - 0.5));
    let glow = (grid_x + grid_z) * depth * u.bass;

    let suv = uv + vec2<f32>(0.0, -0.01 * sign(p.y));
    var c = sample_prev(suv).rgb * 0.9;
    c += hsv2rgb(vec3<f32>(0.6 + p.y * 0.2, 0.9, glow));
    return vec4<f32>(c, 1.0);
}
