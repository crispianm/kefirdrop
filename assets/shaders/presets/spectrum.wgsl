// Frequency spectrum bars rising from the bottom, with light feedback trails.

fn render(uv: vec2<f32>) -> vec4<f32> {
    // Which spectrum bin this column maps to.
    let bin = i32(uv.x * 64.0);
    let mag = spectrum_at(bin);

    // Height from the bottom of the screen (uv.y is 0 at the top).
    let y = 1.0 - uv.y;

    // Colour by frequency, drifting slowly over time.
    let hue = uv.x * 0.8 + u.time * 0.02;
    let col = hsv2rgb(vec3<f32>(hue, 0.85, 1.0));

    // Fill below the bar height, with a brighter cap near the top.
    let fill = step(y, mag);
    let cap = smoothstep(mag, mag - 0.03, y);
    var c = col * fill * (0.45 + 0.55 * cap);

    // Whole-frame flash on the beat.
    c += vec3<f32>(u.beat * 0.12);

    // Feedback: keep a fading ghost of the previous frame.
    let prev = sample_prev(uv).rgb * 0.82;
    return vec4<f32>(max(c, prev), 1.0);
}
