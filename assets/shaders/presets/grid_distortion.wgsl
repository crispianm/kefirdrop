fn render(uv: vec2<f32>) -> vec4<f32> {
    let p = uv - 0.5;
    let r = length(p);
    let bulge = 1.0 - exp(-r * 5.0) * u.beat * 0.5;
    let suv = p * bulge + 0.5;

    var c = sample_prev(suv).rgb * 0.95;
    let grid_x = smoothstep(0.02, 0.0, abs(fract(uv.x * 20.0) - 0.5));
    let grid_y = smoothstep(0.02, 0.0, abs(fract(uv.y * 20.0) - 0.5));
    c += hsv2rgb(vec3<f32>(0.8, 0.8, (grid_x + grid_y) * u.mid * 0.2));

    return vec4<f32>(c, 1.0);
}
