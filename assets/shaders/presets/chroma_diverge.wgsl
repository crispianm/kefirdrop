fn render(uv: vec2<f32>) -> vec4<f32> {
    let p = uv - 0.5;
    let r = length(p);
    let zoom_r = 0.95 - u.bass * 0.05;
    let zoom_g = 0.97 - u.mid * 0.05;
    let zoom_b = 0.99 - u.treble * 0.05;

    let cr = sample_prev(p * zoom_r + 0.5).r;
    let cg = sample_prev(p * zoom_g + 0.5).g;
    let cb = sample_prev(p * zoom_b + 0.5).b;
    var c = vec3<f32>(cr, cg, cb) * 0.94;

    let burst = smoothstep(0.05, 0.0, abs(r - u.beat * 0.5));
    c += vec3<f32>(1.0, 0.8, 0.4) * burst * u.volume;
    return vec4<f32>(c, 1.0);
}
