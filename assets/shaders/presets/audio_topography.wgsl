fn render(uv: vec2<f32>) -> vec4<f32> {
    let p = uv - 0.5;
    let r = length(p);
    let waves = sin(r * 40.0 - u.time * 5.0 + u.bass * 5.0);
    let contour = smoothstep(0.9, 1.0, waves);

    let suv = p * 0.99 + 0.5;
    var c = sample_prev(suv).rgb * 0.9;
    c += hsv2rgb(vec3<f32>(r + u.time * 0.1, 0.8, contour * u.mid));
    return vec4<f32>(c, 1.0);
}
