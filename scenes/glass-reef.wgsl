// name: Glass Reef
// description: A sunlit underwater canyon with drifting coral and a moving shoal.

fn reef_frond(q: vec2f, bend: f32) -> f32 {
    let y = max(0.0, q.y);
    let stem = abs(q.x - bend * y * y) - mix(0.026, 0.005, clamp(y, 0.0, 1.0));
    let sides = vec2f(abs(q.x - bend * y * y), y);
    let leaves = abs(fract(sides.y * 6.5 - 1.9 * sides.x) - 0.5) - (0.16 + 0.12 * (1.0 - y));
    let outline = sides.x - (0.15 - 0.12 * y) * smoothstep(0.04, 0.33, y);
    return max(min(stem, max(leaves * 0.09, outline)), max(-q.y, q.y - 1.0));
}

fn reef_layer(base: vec3f, film: vec2f, seed: f32, depth: f32, ambient: f32) -> vec3f {
    let span = mix(0.100, 0.16, depth);
    let index = floor(film.x / span);
    let bank = smoothstep(0.033, 0.62, abs(film.x + (seed - 0.5) * 0.17));
    let bed = mix(-0.45, -0.29, bank) + 0.032 * fbm(vec2f(film.x * 14.0, depth * 38.0 + seed));
    var color = wf_ink(base, film.y - bed, mix(vec3f(0.065, 0.19, 0.26), vec3f(0.036, 0.114, 0.16), depth), 0.004);
    for (var i = -1; i <= 1; i++) {
        let cell = index + f32(i);
        let rnd = hash(vec2f(cell, depth * 17.0 + seed * 21.0));
        let x = (cell + 0.28 + rnd * 0.39) * span;
        let edge_bank = smoothstep(0.09, 0.77, abs(x));
        let origin_y = bed - 0.041 - rnd * 0.04;
        let height = (0.054 + edge_bank * (0.15 + 0.34 * rnd)) * (0.85 + 0.25 * p.detail);
        let q = (film - vec2f(x, origin_y)) / height;
        if abs(q.x) > 1.35 || q.y < 0.0 || q.y > 1.1 { continue; }
        let drift = sin(p.time * (0.69 + rnd * 0.51) + cell * 0.87) * (0.08 + depth * 0.09);
        var coral = 9.0;
        for (var j = -1; j <= 1; j++) {
            let spread = f32(j);
            let lean = vec2f(spread * 0.34 * q.y, abs(spread) * q.y * -0.28);
            let frond = reef_frond((q - lean) * vec2f(1.0, 1.0 + abs(spread) * 0.31), drift);
            coral = min(coral, frond);
        }
        let ink = mix(vec3f(0.59, 0.30, 0.39), vec3f(0.16, 0.51, 0.46), rnd)
            * (0.42 + 0.46 * smoothstep(0.0, 1.0, q.y) + 0.19 * ambient);
        color = wf_ink(color, coral, ink, 0.012);
    }
    return color;
}

fn reef_fish(base: vec3f, p0: vec2f, seed: f32) -> vec3f {
    let t = p.time;
    let flow = (fract(t * 0.041 + seed * 0.83) - 0.5) * 3.0;
    let q = p0 - vec2f(flow, 0.026 * sin(t * 0.49 + p0.x * 2.1));
    if abs(q.x) > 0.58 || abs(q.y - 0.11) > 0.28 { return base; }
    let grid = (q - vec2f(0.0, 0.10)) * vec2f(24.0, 29.0);
    let tile = floor(grid);
    let local = fract(grid);
    var color = base;
    for (var j = -1; j <= 1; j++) {
        let index = tile + vec2f(f32(j), 0.0);
        let rnd = hash(index + vec2f(seed * 20.5));
        if rnd < 0.44 { continue; }
        let shoal = length(index * vec2f(0.066, 0.18));
        if shoal > 0.91 { continue; }
        let drift = sin(t * (0.98 + rnd * 0.68) + rnd * 18.0);
        let xy = local - vec2f(f32(j) + 0.33 + 0.26 * rnd, 0.53 + 0.18 * drift);
        let scale = 0.65 + rnd * 0.63;
        let k = xy / scale;
        let fish = wf_oval(k, vec2f(0.01, 0.0), vec2f(0.33, 0.13));
        let tail = max(abs(k.y + drift * 0.065) - clamp(-k.x - 0.24, 0.0, 0.23) * 0.89, max(k.x + 0.23, -k.x - 0.53));
        let silver = mix(vec3f(0.10, 0.28, 0.35), vec3f(0.47, 0.70, 0.62), smoothstep(-0.18, 0.20, k.y) * rnd);
        color = wf_ink(color, min(fish, tail), silver, 0.046);
        color = wf_ink(color, length(k - vec2f(0.20, 0.027)) - 0.029, vec3f(0.69, 0.84, 0.68), 0.048);
    }
    return color;
}

fn scene(film: vec2f) -> vec3f {
    let seed = seed_part(19u);
    let q = film + vec2f((p.camera - 0.5) * 0.27, 0.009 * sin(p.time * 0.53));
    let shaft = q.x + q.y * 0.43 + sin(p.time * 0.27) * 0.016;
    let upper = smoothstep(-0.48, 0.51, q.y);
    let bright = exp(-abs(q.x + 0.20) * 2.3) * upper;
    let shafts = pow(max(0.0, sin(shaft * 46.0 + seed * 5.0)), 7.0)
        * exp(-abs(shaft + 0.17) * 3.8)
        * smoothstep(-0.45, 0.3, q.y);
    let deep = vec3f(0.028, 0.093, 0.176);
    let turquoise = vec3f(0.052, 0.43, 0.49);
    var color = mix(deep, turquoise, upper)
        + vec3f(0.22, 0.36, 0.25) * bright * (0.63 + p.light * 0.53)
        + shafts * vec3f(0.14, 0.28, 0.19) * (0.29 + p.light * 0.38);
    let wall = abs(q.x + 0.09 * sin(q.y * 8.0)) - 0.53 + 0.28 * (fbm(q * 10.0 + seed * 31.0) - 0.5);
    let mottled = fbm(q * 35.0 + seed * 21.0);
    let limestone = mix(vec3f(0.057, 0.153, 0.21), vec3f(0.20, 0.47, 0.41), mottled * upper);
    color = mix(color, limestone, smoothstep(-0.02, 0.09, wall) * (0.42 + 0.37 * (1.0 - upper)));
    let surface = 0.415 + sin(q.x * 18.0 + p.time * 1.2) * 0.021;
    let ripples = fbm(q * vec2f(52.0, 190.0) + vec2f(p.time * 0.38, -p.time * 0.3));
    color = mix(color, vec3f(0.23, 0.64, 0.62), smoothstep(surface, surface + 0.15, q.y) * ripples);
    color = reef_fish(color, q, seed);
    color = reef_layer(color, q * 1.13 + vec2f(0.04, 0.09), seed, 0.26, upper);
    color = reef_layer(color, q, seed + 0.33, 0.92, upper);
    if wf_enabled(WF_PARTICLES) {
        let drift = q * vec2f(43.0, 52.0) + vec2f(p.time * 0.38, -p.time * 0.8);
        let tile = floor(drift);
        let dot_center = 0.20 + 0.60 * vec2f(hash(tile + seed * 13.0), hash(tile + 9.0));
        let mote = exp(-length(fract(drift) - dot_center) * 21.0) * smoothstep(0.74, 1.0, hash(tile));
        color += mote * vec3f(0.25, 0.51, 0.42) * (0.30 + p.detail * 0.60);
    }
    return color;
}
