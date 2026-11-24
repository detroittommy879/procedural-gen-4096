// Story characters are drawn in camera space so they can appear in custom scenes.
fn wf_box(q: vec2f, center: vec2f, extent: vec2f, radius: f32) -> f32 {
    let d = abs(q - center) - extent + radius;
    return length(max(d, vec2f(0.0))) + min(max(d.x, d.y), 0.0) - radius;
}

fn wf_oval(q: vec2f, center: vec2f, radius: vec2f) -> f32 {
    return (length((q - center) / radius) - 1.0) * min(radius.x, radius.y);
}

fn wf_ink(base: vec3f, shape: f32, ink: vec3f, edge: f32) -> vec3f {
    return mix(base, ink, 1.0 - smoothstep(-edge, edge, shape));
}

fn wf_cell(color_in: vec3f, q: vec2f, edge: f32) -> vec3f {
    let pulse = 0.78 + 0.22 * sin(p.time * 5.0);
    var color = color_in + vec3f(0.07, 0.65, 0.34) * pulse * exp(-length(q) * 4.8);
    let gem = abs(q.x) * 1.28 + abs(q.y);
    color = wf_ink(color, gem - 0.24, vec3f(0.12, 0.77, 0.51), edge);
    return wf_ink(color, gem - 0.13, vec3f(0.73, 1.04, 0.70) * pulse, edge);
}

fn wf_mica(color_in: vec3f, at: vec2f, sway: f32, stride: f32, direction: f32, scale: f32, cell: f32) -> vec3f {
    if abs(at.x) > 1.8 || at.y > 1.8 || at.y < -0.35 { return color_in; }
    let edge = 0.009 + 0.58 / (min(p.size.x, p.size.y) * scale);
    let shadow = exp(-dot(at * vec2f(1.22, 7.2), at * vec2f(1.22, 7.2)) * 1.42);
    let q = vec2f(at.x * direction - sway * 0.028, at.y - abs(sway) * 0.075);
    let shell_dark = vec3f(0.18, 0.20, 0.26);
    let shell_light = vec3f(0.76, 0.57, 0.39);
    let edge_color = vec3f(0.055, 0.070, 0.090);
    var color = color_in;
    if wf_enabled(WF_SHADOWS) { color *= 1.0 - shadow * 0.64; }

    let track = wf_box(q, vec2f(-0.08, 0.205), vec2f(0.69, 0.20), 0.13);
    let tread = 0.5 + 0.5 * sin(q.x * 42.0 + stride);
    let rubber = mix(vec3f(0.063, 0.082, 0.096), vec3f(0.18, 0.21, 0.23), tread * 0.57);
    color = wf_ink(color, track - 0.029, edge_color, edge);
    color = wf_ink(color, track + 0.019, rubber, edge);
    for (var i = 0; i < 3; i++) {
        let x = f32(i) * 0.31 - 0.45;
        let hub = wf_oval(q, vec2f(x, 0.20), vec2f(0.105, 0.102));
        color = wf_ink(color, hub, vec3f(0.30, 0.35, 0.38), edge);
        color = wf_ink(color, hub + 0.052, vec3f(0.09, 0.11, 0.13), edge);
    }

    let neck = wf_box(q, vec2f(-0.07, 0.52), vec2f(0.20, 0.25), 0.06);
    color = wf_ink(color, neck, vec3f(0.36, 0.40, 0.43), edge);
    let shoulder = wf_oval(q, vec2f(-0.63, 0.69), vec2f(0.16, 0.26));
    color = wf_ink(color, shoulder, vec3f(0.31, 0.35, 0.38), edge);
    let body = wf_box(q, vec2f(-0.04, 0.80), vec2f(0.65, 0.40), 0.20);
    let metal = mix(shell_dark, shell_light, smoothstep(0.40, 1.23, q.y + q.x * 0.19));
    color = wf_ink(color, body - 0.033, edge_color, edge);
    color = wf_ink(color, body, metal, edge);
    let ridge = max(abs(body + 0.052) - 0.018, 1.05 - q.y);
    color = wf_ink(color, ridge, vec3f(0.92, 0.77, 0.51), edge);

    let glass = wf_box(q, vec2f(0.11, 0.83), vec2f(0.44, 0.25), 0.15);
    color = wf_ink(color, glass - 0.021, vec3f(0.092, 0.115, 0.15), edge);
    let visor = mix(vec3f(0.024, 0.084, 0.113), vec3f(0.16, 0.34, 0.41), smoothstep(0.61, 1.14, q.y - q.x * 0.27));
    color = wf_ink(color, glass + 0.026, visor, edge);
    let look = vec2f(0.20 + 0.057 * sin(p.time * 0.81), 0.835 + sway * 0.027);
    let blink = 1.0 - 0.88 * smoothstep(0.956, 0.998, cos(p.time * 1.56 + 1.4));
    let eye = wf_oval(q, look, vec2f(0.127, 0.121 * blink));
    color = wf_ink(color, eye, vec3f(0.26, 0.84, 0.74), edge);
    color = wf_ink(color, wf_oval(q, look + vec2f(-0.028, 0.025 * blink), vec2f(0.037, 0.029 * blink)),
                    vec3f(0.91, 1.02, 0.91), edge);
    let diodes = wf_oval(q, vec2f(-0.47, 0.92), vec2f(0.034));
    color = wf_ink(color, diodes, vec3f(1.08, 0.53, 0.22), edge);

    let mast = max(abs(q.x + 0.32 + (q.y - 1.15) * 0.23) - 0.025, abs(q.y - 1.28) - 0.15);
    color = wf_ink(color, mast, vec3f(0.37, 0.41, 0.43), edge);
    let tip = q - vec2f(-0.38, 1.43);
    color += vec3f(0.77, 0.29, 0.06) * exp(-dot(tip, tip) * 78.0) * 0.43;
    color = wf_ink(color, length(tip) - 0.056, vec3f(0.99, 0.60, 0.24), edge);
    if cell > 0.0 {
        let link = wf_box(q, vec2f(0.62, 0.52), vec2f(0.21, 0.054), 0.031);
        color = wf_ink(color, link, vec3f(0.46, 0.43, 0.35), edge);
        color = wf_cell(color, q - vec2f(0.79, 0.58), edge);
    }
    return color;
}

fn wf_kite(color_in: vec3f, at: vec2f, tilt: f32, scale: f32, scanning: f32) -> vec3f {
    if abs(at.x) > 1.9 || at.y > 1.01 || at.y < -3.9 { return color_in; }
    let edge = 0.009 + 0.53 / (min(p.size.x, p.size.y) * scale);
    let q = vec2f(at.x + at.y * tilt, at.y - at.x * tilt);
    let cone = max(0.0, 1.0 - abs(q.x) / (0.12 + 0.31 * abs(q.y)));
    let beam = cone * exp(q.y * 0.84) * smoothstep(-3.7, -0.60, q.y) * (1.0 - smoothstep(-0.6, -0.1, q.y));
    var color = color_in + scanning * beam * vec3f(0.028, 0.21, 0.32);
    let wing_q = vec2f(abs(q.x), q.y + abs(q.x) * 0.19);
    let wing = wf_box(wing_q, vec2f(0.69, 0.10), vec2f(0.54, 0.090), 0.072);
    color = wf_ink(color, wing, vec3f(0.40, 0.57, 0.63), edge);
    let rotor = wf_oval(wing_q, vec2f(0.95, 0.20), vec2f(0.34, 0.058));
    color = wf_ink(color, rotor, mix(vec3f(0.11, 0.26, 0.30), vec3f(0.36, 0.69, 0.71), 0.5 + 0.32 * sin(p.time * 32.0)), edge);
    let body = wf_oval(q, vec2f(0.0, 0.08), vec2f(0.52, 0.32));
    color = wf_ink(color, body - 0.028, vec3f(0.036, 0.086, 0.10), edge);
    color = wf_ink(color, body, mix(vec3f(0.13, 0.26, 0.31), vec3f(0.61, 0.77, 0.74), smoothstep(-0.18, 0.35, q.y)), edge);
    let lens = wf_oval(q, vec2f(0.0, 0.02), vec2f(0.28, 0.17));
    color = wf_ink(color, lens, vec3f(0.027, 0.13, 0.18), edge);
    let glow = length(q - vec2f(0.0, -0.017));
    color += exp(-glow * 6.4) * vec3f(0.08, 0.34, 0.44);
    return wf_ink(color, glow - 0.087, vec3f(0.44, 0.95, 1.10), edge);
}

fn wf_story_frame(film: vec2f) -> vec3f {
    var color = scene(film);
    if p.action == 0u { return color; }

    let progress = smoothstep(0.02, 0.98, p.progress);
    let pulse = sin(p.time * 1.8);
    var x = mix(-0.42, -0.08, progress);
    var y = -0.34;
    var drone = vec2f(0.17 + 0.047 * sin(p.time * 0.27), 0.18 + 0.035 * cos(p.time * 0.43));
    var scan = 0.30;
    var carry = 0.0;
    var direction = 1.0;
    switch p.action {
        case 1u: {
            x = mix(-0.35, -0.075, smoothstep(0.08, 0.64, progress));
            scan = smoothstep(0.42, 0.68, progress) * 0.92;
            drone.x += mix(0.18, -0.10, progress);
        }
        case 2u: {
            x = mix(-0.48, -0.14, progress);
            drone.x -= 0.018;
            scan = 0.88 * (0.44 + 0.56 * smoothstep(0.3, 0.96, sin(p.time * 5.0)));
        }
        case 3u: {
            x = mix(-0.59, 0.48, progress);
            y += 0.041 * sin(progress * 3.14);
            drone.x += 0.084 * cos(progress * TAU);
        }
        case 4u: {
            x = mix(-0.38, 0.16, progress);
            y += 0.058 + 0.015 * pulse;
            drone.y -= 0.11 * smoothstep(0.53, 0.99, progress);
            scan = 0.41 + progress * 0.54;
        }
        case 5u: {
            x = mix(-0.40, -0.032, smoothstep(0.01, 0.82, progress));
            y += 0.043 * pulse + 0.099;
            carry = step(0.74, p.progress);
            let gem = (film - vec2f(0.115, -0.235 + 0.016 * sin(p.time * 2.0))) / 0.11;
            if carry == 0.0 && length(gem) < 2.0 { color = wf_cell(color, gem, 0.028); }
            scan = 0.75;
        }
        case 6u: {
            x = -0.12 + 0.06 * sin(progress * TAU);
            y += 0.116 + 0.02 * pulse;
            drone.y += 0.039;
            carry = 1.0;
            scan = 0.0;
        }
        case 7u: {
            x = mix(-0.40, 0.31, smoothstep(0.05, 0.69, progress));
            direction = select(1.0, -1.0, p.progress > 0.84);
            y -= 0.014;
            carry = 1.0 - step(0.54, p.progress);
            scan = 0.85 * (1.0 - step(0.54, p.progress));
            drone.x -= 0.16 * smoothstep(0.60, 0.95, progress);
        }
        default: {}
    }
    let width = min(1.0, p.size.x / min(p.size.x, p.size.y) * 0.76);
    let at = vec2f(x * width, y);
    let ortho = 0.107;
    let flight = vec2f((x + drone.x) * width, y + drone.y + 0.088);
    color = wf_mica(color, (film - at) / ortho, pulse, p.time * 14.0, direction, ortho, carry);
    return wf_kite(color, (film - flight) / 0.066, 0.26 * sin(p.time * 0.87), 0.066, scan);
}
