// name: Red Arch
// description: A sandstone arch and layered mesas in warm desert light.

fn sand_height(q: vec2f, variant: f32) -> f32 {
    let apron = exp(-dot(q * vec2f(0.118, 0.068), q * vec2f(0.118, 0.068))) * 0.34;
    let waves = sin(q.x * 0.17 + q.y * 0.23) * 0.28 + sin(q.x * 0.43 - q.y * 0.081) * 0.19;
    return apron + waves + (fbm(q * 0.13 + variant * 19.0) - 0.5) * 1.02;
}

fn mesa(q: vec2f, center: vec2f, radius: vec2f, height: f32, variant: f32) -> f32 {
    let x = abs((q - center) / radius);
    let region = max(x.x * 0.84, x.y * 0.84) + 0.27 * length(x);
    let rough = (noise(q * 0.29 + variant * 7.0) - 0.5) * 0.16;
    let wall = 1.0 - smoothstep(0.49, 0.98, region + rough);
    let apron = max(0.0, 1.14 - region) * 0.16;
    return height * (wall * 0.87 + apron);
}

fn desert_height(q: vec2f, variant: f32) -> f32 {
    let mesas = max(
        max(mesa(q, vec2f(-27.0, 43.0), vec2f(23.0, 17.0), 8.4, variant),
            mesa(q, vec2f(33.0, 59.0), vec2f(18.0, 19.0), 12.8, variant)),
        max(mesa(q, vec2f(-2.0, 76.0), vec2f(16.0, 23.0), 13.2, variant),
            mesa(q, vec2f(52.0, 90.0), vec2f(23.0, 26.0), 17.2, variant))
    );
    return sand_height(q, variant) + mesas;
}

fn arch_distance(point: vec3f, variant: f32) -> f32 {
    let skew = mix(0.18, 0.48, variant);
    let q = point - vec3f(0.1, 3.38, 7.1);
    let x = q.x - q.y * skew * 0.26 + 0.20 * sin(q.y * 0.86 + variant * 3.0);
    let z = q.z + q.x * skew;
    let oval = length(vec2f(x / (4.06 + variant * 0.40), max(q.y, 0.0) / 3.8));
    let rim = (oval - 1.0) * 3.8;
    let width = 0.76 + 0.31 * smoothstep(-1.8, 3.0, x) + 0.28 * smoothstep(0.25, 3.0, -q.y);
    let depth = 0.87 + 0.28 * smoothstep(0.0, 4.4, x);
    let bend = 0.31 * sin(x * 0.87) + 0.22 * sin(q.y * 0.84 + variant * 3.0);
    let outline = length(vec2f(rim / width, (z - bend) / depth)) - 1.0;
    let foot = max(abs(x - 4.18) - 1.35, abs(z - 0.12) - 1.18);
    let columns = min(outline * 0.74, max(foot * 0.65, q.y + 2.03));
    let weather = (fbm(vec2f(point.x + point.z * 0.56, point.y * 1.76) * 2.1 + variant * 42.0) - 0.5) * 0.23;
    return columns + weather;
}

fn desert_map(point: vec3f, variant: f32) -> vec2f {
    let ground = (point.y - desert_height(point.xz, variant)) * 0.58;
    if abs(point.z - 7.1) > 5.5 || abs(point.x) > 7.5 || point.y > 8.8 {
        return vec2f(min(ground, 4.5), 0.0);
    }
    let arch = arch_distance(point, variant);
    return select(vec2f(ground, 0.0), vec2f(arch, 1.0), arch < ground);
}

fn desert_trace(eye: vec3f, rd: vec3f, variant: f32) -> vec2f {
    var t = 0.06;
    for (var i = 0; i < 115; i++) {
        let point = eye + rd * t;
        let probe = desert_map(point, variant);
        if probe.x < 0.012 + t * 0.00055 { return vec2f(t, probe.y); }
        t += clamp(probe.x * 0.85, 0.066 + t * 0.004, 4.2);
        if t > 123.0 { break; }
    }
    return vec2f(-1.0);
}

fn scene(film: vec2f) -> vec3f {
    let variant = seed_part(3u);
    let eye = vec3f(mix(-5.3, 4.8, p.camera), mix(1.78, 3.6, p.camera), -10.8);
    let rd = ray(eye, vec3f(0.65, 3.05, 8.4), film, 1.14);
    let sun = normalize(vec3f(-0.62, mix(0.16, 0.91, p.light), -0.25));
    let sky_sun = normalize(vec3f(-0.56, mix(0.10, 0.85, p.light), 0.83));
    let sky = atmosphere(rd, sky_sun, vec3f(0.13, 0.38, 0.63), variant * 21.0);
    let hit = desert_trace(eye, rd, variant);
    let haze = mix(vec3f(0.73, 0.46, 0.42), vec3f(0.51, 0.63, 0.73), p.light);
    if hit.x < 0.0 {
        return mix(haze, sky, smoothstep(-0.031, 0.010, rd.y));
    }

    let point = eye + rd * hit.x;
    let eps = 0.036 + hit.x * 0.001;
    let normal = normalize(vec3f(
        desert_map(point + vec3f(eps, 0.0, 0.0), variant).x - desert_map(point - vec3f(eps, 0.0, 0.0), variant).x,
        desert_map(point + vec3f(0.0, eps, 0.0), variant).x - desert_map(point - vec3f(0.0, eps, 0.0), variant).x,
        desert_map(point + vec3f(0.0, 0.0, eps), variant).x - desert_map(point - vec3f(0.0, 0.0, eps), variant).x
    ));
    let variations = fbm(point.xz * 0.67 + point.y * 0.50 + variant * 31.0);
    let veins = point.y + point.x * 0.09 + 0.16 * sin(point.z * 0.51) + 0.24 * variations;
    let bands = 0.55 * noise(vec2f(veins * 3.1, variant * 20.0))
              + 0.30 * noise(vec2f(veins * 11.0, 9.0))
              + 0.15 * noise(vec2f(veins * 28.0, 6.0));
    let pebble = noise(point.xz * mix(12.0, 32.0, p.detail) + point.y * 8.0);
    let dune = 0.5 + 0.5 * sin(point.x * 8.0 + point.z * 3.1 + variations * 8.0);
    let sand = mix(vec3f(0.41, 0.215, 0.126), vec3f(0.87, 0.57, 0.34), 0.29 + variations * 0.51 + 0.14 * pebble + 0.055 * dune);
    let rock = mix(vec3f(0.30, 0.083, 0.054), vec3f(0.92, 0.49, 0.27), smoothstep(0.10, 0.93, bands * 0.91 + variations * 0.25));
    let base = mix(sand, rock, max(hit.y, (1.0 - smoothstep(0.55, 0.92, normal.y)) * 0.82));
    var shade = 1.0;
    if wf_enabled(WF_SHADOWS) {
        var shadow_t = 0.13;
        for (var i = 0; i < 17; i++) {
            let probe = desert_map(point + normal * 0.09 + sun * shadow_t, variant).x;
            shade = min(shade, 7.0 * max(0.0, probe) / shadow_t);
            if shade < 0.06 || shadow_t > 18.0 { break; }
            shadow_t += clamp(probe * 0.85, 0.12, 2.5);
        }
    }
    let direct = max(dot(normal, sun), 0.0) * shade;
    let sun_tint = mix(vec3f(1.50, 1.02, 0.65), vec3f(1.18, 1.08, 0.96), p.light);
    let ambient = vec3f(0.56, 0.56, 0.67) * (0.72 + 0.28 * normal.y);
    let lit = base * (ambient + sun_tint * direct * 0.86);
    var color = mix(lit, haze, 1.0 - exp(-hit.x * 0.0062));
    if wf_enabled(WF_PARTICLES) {
        let gust = fbm(film * vec2f(7.9, 41.0) + vec2f(-p.time * 0.48, variant * 23.0));
        let near_ground = exp(-abs(film.y + 0.13 + film.x * 0.11) * 14.0);
        color = mix(color, haze * vec3f(1.1, 1.01, 0.92), near_ground * smoothstep(0.48, 0.78, gust) * 0.24);
    }
    return color;
}
