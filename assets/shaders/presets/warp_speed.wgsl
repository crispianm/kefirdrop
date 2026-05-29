fn render(uv: vec2<f32>) -> vec4<f32> {
    let p = uv - 0.5;
    let suv = p * (0.9 + u.bass * 0.05) + 0.5;
    var c = sample_prev(suv).rgb * 0.9;

    let seed = fract(sin(dot(uv, vec2<f32>(12.9898, 78.233))) * 43758.5453);
    let spawn = smoothstep(0.995 - u.volume * 0.01, 1.0, seed);

    c += vec3<f32>(spawn);
    return vec4<f32>(c, 1.0);
}
