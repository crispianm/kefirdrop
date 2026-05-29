fn render(uv: vec2<f32>) -> vec4<f32> {
    let t = u.time;
    let p = (uv - 0.5) * 10.0;
    let v1 = sin(p.x + t);
    let v2 = sin(p.y + t);
    let v3 = sin(p.x + p.y + t);
    let r = length(p);
    let v4 = sin(r + t);

    let v = v1 + v2 + v3 + v4;
    let ring = smoothstep(0.1, 0.0, abs(v / 4.0 - u.mid));

    let suv = (uv - 0.5) * 0.99 + 0.5;
    var c = sample_prev(suv).rgb * 0.92;
    c += hsv2rgb(vec3<f32>(v * 0.1 + t * 0.1, 0.8, ring));
    return vec4<f32>(c, 1.0);
}
