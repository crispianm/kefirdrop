// Audio-modulated plasma: layered sine fields whose amplitude swells with the
// bass and whose hue shifts with the mids. A touch of feedback softens motion.

fn render(uv: vec2<f32>) -> vec4<f32> {
    let t = u.time;
    let p = uv * 4.0;

    var v = 0.0;
    v += sin(p.x + t);
    v += sin((p.y + t) * 0.7);
    v += sin((p.x + p.y + t) * 0.5);
    let cx = p.x + 0.5 * sin(t * 0.3);
    let cy = p.y + 0.5 * cos(t * 0.4);
    v += sin(sqrt(cx * cx + cy * cy) * 2.0 + t);

    // Bass swells the field; mids drift the palette; beat brightens.
    v *= 0.5 + u.bass * 1.5;
    let hue = fract(v * 0.1 + t * 0.02 + u.mid * 0.2);
    let col = hsv2rgb(vec3<f32>(hue, 0.8, 0.55 + u.volume * 0.6 + u.beat * 0.3));

    let prev = sample_prev(uv).rgb;
    return vec4<f32>(mix(col, prev, 0.3), 1.0);
}
