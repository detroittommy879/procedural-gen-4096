// name: Lavender Fields
// description: Purple rows converge on a solitary tree at the crest of a hill.

fn rows_axis(q: vec2f, variant: f32) -> f32 {
    return q.x + sin(q.y * 0.068 + variant * 2.0) * 0.72;
}

fn rows_ground(q: vec2f, variant: f32) -> f32 {
    let rolling = 2.65 * exp(-pow((q.y - 38.0) * 0.036, 2.0));
    let pitch = 0.23 * sin(q.x * 0.20 + q.y * 0.07) + 0.08 * sin(q.y * 0.57 + q.x * 0.54);
    return rolling + pitch + 0.31 * exp(-pow(q.x * 0.18, 2.0));
}

fn rows_bloom(q: vec2f, variant: f32) -> f32 {
    let x = rows_axis(q, variant);
    let spacing = 1.36 + variant * 0.20;
    let ridge = abs(fract(x / spacing) * 2.0 - 1.0);
    let slope = 1.0 - smoothstep(0.26, 0.93, ridge);
    let last_row = 1.0 - smoothstep(36.0, 47.0, q.y + abs(q.x) * 0.045);
    return slope * last_row;
}

fn rows_land(q: vec2f, variant: f32) -> f32 {
    let wind = vec2f(sin(p.time * 1.1) * 0.21, cos(p.time * 0.81) * 0.15 - 0.15);
    let cluster = noise(q * vec2f(2.8, 3.9) + variant * 9.0 + wind);
    return rows_ground(q, variant) + rows_bloom(q, variant) * (0.35 + cluster * 0.22);
}

fn rows_trace(ro: vec3f, rd: vec3f, variant: f32) -> f32 {
    var t = 0.05;
    var last = t;
    for (var i = 0; i < 108; i++) {
        let hit = ro + t * rd;
        let gap = hit.y - rows_land(hit.xz, variant);
        if gap < 0.0 {
            var low = last;
            var high = t;
            for (var j = 0; j < 5; j++) {
                let mid = (low + high) * 0.5;
                let point = ro + mid * rd;
                if point.y > rows_land(point.xz, variant) { low = mid; } else { high = mid; }
            }
            return (low + high) * 0.5;
        }
        last = t;
        t += clamp(gap * 0.68, 0.096 + t * 0.004, 4.5);
        if t > 90.0 { break; }
    }
    return -1.0;
}

fn branch_distance(q: vec3f, a: vec3f, b: vec3f, r0: f32, r1: f32) -> f32 {
    let axis = b - a;
    let t = clamp(dot(q - a, axis) / dot(axis, axis), 0.0, 1.0);
    return length(q - a - axis * t) - mix(r0, r1, t);
}

// X stores the distance and Y distinguishes bark from foliage.
fn rows_tree(q: vec3f, spread: f32) -> vec2f {
    var bark = branch_distance(q, vec3f(0.0, -0.28, 0.0), vec3f(0.14, 1.94, 0.04), 0.14, 0.063);
    bark = min(bark, branch_distance(q, vec3f(0.07, 1.18, 0.0), vec3f(-1.04, 2.59, -0.06), 0.078, 0.019));
    bark = min(bark, branch_distance(q, vec3f(0.12, 1.57, 0.04), vec3f(1.14, 2.82, 0.06), 0.068, 0.021));
    bark = min(bark, branch_distance(q, vec3f(0.13, 1.8, 0.05), vec3f(-0.12, 3.28, 0.12), 0.058, 0.018));
    let a = length((q - vec3f(-0.86 * spread, 2.74, 0.10)) / vec3f(1.00, 0.68, 0.71)) - 1.0;
    let b = length((q - vec3f(0.93 * spread, 2.91, 0.18)) / vec3f(0.94, 0.63, 0.78)) - 1.0;
    let c = length((q - vec3f(-0.10, 3.31, 0.15)) / vec3f(1.08, 0.79, 0.78)) - 1.0;
    let d = length((q - vec3f(0.48, 3.34, -0.37)) / vec3f(0.84, 0.64, 0.67)) - 1.0;
    let leaf_noise = noise(q.xz * 7.0 + q.y * vec2f(3.0, -4.0)) - 0.50;
    let canopy = min(min(a, b), min(c, d)) * 0.62 + leaf_noise * 0.27;
    return select(vec2f(canopy, 1.0), vec2f(bark, 0.0), bark < canopy);
}

fn tree_hit(ro: vec3f, rd: vec3f, center: vec3f, spread: f32) -> vec2f {
    let oc = ro - center - vec3f(0.0, 1.9, 0.0);
    let b = dot(oc, rd);
    let discriminant = b * b - dot(oc, oc) + 7.84;
    if discriminant <= 0.0 { return vec2f(-1.0); }
    let disk = sqrt(discriminant);
    var t = max(0.0, -b - disk);
    for (var i = 0; i < 46; i++) {
        let probe = rows_tree(ro + rd * t - center, spread);
        if probe.x < 0.016 { return vec2f(t, probe.y); }
        t += max(0.020, probe.x * 0.68);
        if t > -b + disk { break; }
    }
    return vec2f(-1.0);
}

fn scene(film: vec2f) -> vec3f {
    let variant = seed_part(9u);
    let eye = vec3f(mix(-1.4, 1.4, p.camera), mix(2.0, 3.7, p.camera), -3.5);
    let rd = ray(eye, vec3f(0.0, 5.1, 36.0), film, 1.25);
    let sun = sunlight();
    let sky = atmosphere(rd, sun, vec3f(0.14, 0.36, 0.72), variant * 19.0);
    let horizon = 0.045 + fbm(vec2f(rd.x * 12.0, variant * 9.0)) * 0.086;
    let distant = mix(vec3f(0.48, 0.55, 0.69), vec3f(0.63, 0.65, 0.73), p.light);
    let background = mix(distant, sky, smoothstep(horizon - 0.004, horizon, rd.y));

    let t = rows_trace(eye, rd, variant);
    let base = vec2f(mix(-0.7, 0.7, variant), 36.0);
    let tree = vec3f(base.x, rows_ground(base, variant), base.y);
    let spread = 0.80 + seed_part(13u) * 0.34;
    let tree_result = tree_hit(eye, rd, tree, spread);
    if tree_result.x > 0.0 && (t < 0.0 || tree_result.x < t) {
        let q = eye + rd * tree_result.x - tree;
        let e = 0.033;
        let normal = normalize(vec3f(
            rows_tree(q + vec3f(e, 0.0, 0.0), spread).x - rows_tree(q - vec3f(e, 0.0, 0.0), spread).x,
            rows_tree(q + vec3f(0.0, e, 0.0), spread).x - rows_tree(q - vec3f(0.0, e, 0.0), spread).x,
            rows_tree(q + vec3f(0.0, 0.0, e), spread).x - rows_tree(q - vec3f(0.0, 0.0, e), spread).x
        ));
        let tex = noise(q.xz * 17.0 + q.y * 16.0);
        let leaves = mix(vec3f(0.10, 0.18, 0.11), vec3f(0.49, 0.56, 0.21), tex);
        let bark = vec3f(0.25, 0.17, 0.13);
        return mix(bark, leaves, tree_result.y) * (0.58 + 0.94 * max(dot(normal, sun), 0.0)) + distant * 0.12;
    }
    if t < 0.0 { return background; }

    let point = eye + rd * t;
    let q = point.xz;
    let bloom = rows_bloom(q, variant);
    let plant_scale = mix(11.0, 24.0, p.detail);
    let wind = vec2f(sin(p.time * 1.1 + q.y * 0.39) - sin(q.y * 0.39), sin(p.time * 0.83));
    let petal = noise(q * vec2f(plant_scale, plant_scale * 1.36) + variant * 51.0 + wind * 0.21);
    let cluster = fbm(q * 2.7 + variant * 73.0);
    let stems = noise((q + vec2f(petal * 0.02, 0.0)) * vec2f(plant_scale * 2.9, plant_scale * 0.77));
    let flowers_detail = smoothstep(0.42, 0.85, stems * 0.67 + petal * 0.38);
    let soil = mix(vec3f(0.33, 0.18, 0.11), vec3f(0.85, 0.65, 0.39), 0.29 + cluster * 0.48 + petal * 0.18);
    let flowers = mix(
        vec3f(0.19, 0.065, 0.32),
        mix(vec3f(0.75, 0.34, 0.69), vec3f(0.53, 0.39, 0.91), seed_part(17u)),
        smoothstep(0.15, 0.87, 0.61 * petal + 0.40 * cluster) * 0.83 + flowers_detail * 0.17
    );
    let mask = smoothstep(0.11, 0.43, bloom + (petal - 0.5) * 0.19);
    let slope = clamp((rows_land(q - vec2f(0.13, 0.0), variant) - rows_land(q + vec2f(0.13, 0.0), variant)) * 1.63, -0.55, 0.55);
    let light_color = mix(vec3f(1.26, 1.03, 0.85), vec3f(1.03, 1.07, 1.11), p.light);
    let lit = mix(soil, flowers, mask) * (0.74 - slope * 0.33 + 0.18 * sun.y) * light_color;
    var color = mix(lit, distant, 1.0 - exp(-t * 0.0055));
    if wf_enabled(WF_PARTICLES) {
        let drift = film * 73.0 + vec2f(-0.52, -0.70) * p.time;
        let grid = floor(drift);
        let center = 0.17 + 0.66 * vec2f(hash(grid + 41.0), hash(grid + 17.0));
        let pollen = exp(-length(fract(drift) - center) * 26.0) * smoothstep(0.87, 1.0, hash(grid));
        color += vec3f(0.67, 0.48, 0.27) * pollen * 0.39;
    }
    return color;
}
