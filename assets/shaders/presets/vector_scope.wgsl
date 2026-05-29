fn render(uv: vec2<f32>) -> vec4<f32> {
    let t = u.time * 2.0;
    let lx = sin(t * 3.0) * 0.4;
    let ly = sin(t * 4.0 + 1.57) * 0.4;
    let lissajous = vec2<f32>(lx, ly) + 0.5;

    let d = length(uv - lissajous);
    let dot = smoothstep(0.03 + u.bass * 0.05, 0.0, d);

    let suv = (uv - 0.5) * 0.98 + 0.5;
    var c = sample_prev(suv).rgb * 0.97;
    c += hsv2rgb(vec3<f32>(u.time * 0.3, 0.9, dot));
    return vec4<f32>(c, 1.0);
}
