// name: Neon Passage
// description: Twisting neon frames over a reflective floor and a field of sparks.

fn neon_color(phase: f32) -> vec3f {
    return 0.51 + 0.49 * cos(phase + vec3f(0.0, 2.05, 4.20));
}

fn neon_rotate(q: vec2f, a: f32) -> vec2f {
    return vec2f(cos(a) * q.x - sin(a) * q.y, sin(a) * q.x + cos(a) * q.y);
}

fn neon_shape(q: vec2f, style: f32) -> f32 {
    let a = abs(q);
    let diamond = (a.x + a.y) * 0.7071;
    let square = max(a.x, a.y);
    let hex = max(a.y, dot(a, vec2f(0.866, 0.5)));
    return mix(mix(square, hex, smoothstep(0.08, 0.61, style)), diamond, smoothstep(0.72, 1.0, style));
}

fn neon_frames(eye: vec3f, rd: vec3f, phase: f32, twist: f32, floor_y: f32) -> vec3f {
    var result = vec3f(0.0);
    for (var i = 0; i < 24; i++) {
        let n = f32(i);
        let depth = 1.1 + n * 1.18;
        if depth <= eye.z { continue; }
        let travel = (depth - eye.z) / max(0.01, rd.z);
        let point = eye + rd * travel;
        if point.y < floor_y { break; }
        let center = vec2f(sin(n * 0.25 + phase) * 0.28, cos(n * 0.21) * 0.17);
        let q = neon_rotate(point.xy - center, n * twist + phase * 0.27
            + 0.16 * (sin(p.time * 0.51 - n * 0.29) - sin(-n * 0.29)));
        let edge = abs(neon_shape(q, seed_part(32u)) - (2.24 + 0.13 * sin(n * 0.49 + phase)));
        let aa = 0.0035 + travel / min(p.size.x, p.size.y) * 0.4;
        let filament = 1.0 - smoothstep(0.004 + aa * 0.2, 0.017 + aa, edge);
        let halo = exp(-edge * mix(25.0, 14.0, p.detail));
        let aura = exp(-edge * 5.7);
        let palette = neon_color(phase + n * 0.15 + p.color * TAU + p.time * 0.13);
        let hue = mix(vec3f(0.10, 0.30, 1.0), palette, 0.77);
        let attenuation = exp(-travel * 0.052);
        result += attenuation * (hue * (1.25 * filament + 0.49 * halo + 0.068 * aura)
                              + vec3f(1.0, 0.94, 0.86) * filament * 0.36);
    }
    return result;
}

fn scene(film: vec2f) -> vec3f {
    let phase = seed_part(5u) * TAU;
    let twist = mix(-0.19, 0.19, p.camera) + sin(phase) * 0.065;
    let eye = vec3f(0.36 * sin(phase), 0.19, -3.0);
    let rd = ray(eye, vec3f(0.20 * cos(phase), -0.06, 8.0), film, 0.84);
    let floor_y = -1.48;
    let uv = rd.xy / rd.z;
    let center = exp(-length(uv * vec2f(0.7, 1.1)) * 4.4);
    let palette = neon_color(phase + p.color * TAU + 1.0);
    var color = vec3f(0.008, 0.010, 0.024) + center * (0.024 + 0.14 * p.light) * (palette * 0.66 + 0.25);
    color += neon_frames(eye, rd, phase, twist, floor_y);

    if wf_enabled(WF_REFLECTIONS) && rd.y < -0.0001 {
        let t = (floor_y - eye.y) / rd.y;
        let point = eye + t * rd;
        if t < 29.0 {
            let ripple = (noise(point.xz * vec2f(22.0, 13.0) + vec2f(p.time * 0.97, p.time * 1.7)) - 0.5) * 0.018;
            let reflected = reflect(rd, normalize(vec3f(ripple, 1.0, ripple * 1.7)));
            let hit = point + vec3f(0.0, 0.01, 0.0);
            let mirrored = neon_frames(hit, reflected, phase, twist, floor_y);
            let fade = exp(-t * 0.063);
            let grid = abs(fract(point.xz * vec2f(1.65, 0.84) + 0.5) - 0.5);
            let line = exp(-min(grid.x, grid.y) * 110.0) * 0.040;
            color += (mirrored * (0.39 + line) + palette * line) * fade;
        }
    }

    if !wf_enabled(WF_PARTICLES) { return color; }
    let dots = uv * 107.0 + vec2f(-0.18, -0.95) * p.time;
    let tile = floor(dots);
    let dart = vec2f(hash(tile + phase), hash(tile + phase + 8.0)) * 0.76 + 0.12;
    let spark = exp(-length(fract(dots) - dart) * 21.0);
    let scattered = smoothstep(0.91, 1.0, hash(tile + phase + 33.0));
    return color + spark * scattered * (0.15 + 0.65 * p.detail) * (palette + 0.33);
}
