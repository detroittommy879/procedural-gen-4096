// name: Alpine Lake
// description: A glacial lake beneath a jagged, snow-dusted ridge.

fn peak_land(q: vec2f, center: vec2f, scale: vec2f, height: f32) -> f32 {
    let a = (q - center) / scale;
    return height * max(0.0, 1.0 - length(a));
}

fn lake_land(q: vec2f, variant: f32) -> f32 {
    let shift = (variant - 0.5) * 7.0;
    var silhouette = peak_land(q, vec2f(-18.0 + shift, 43.0), vec2f(24.0, 27.0), 18.4);
    silhouette = max(silhouette, peak_land(q, vec2f(-6.0 + shift, 55.0), vec2f(21.0, 32.0), 23.5));
    silhouette = max(silhouette, peak_land(q, vec2f(8.0 + shift, 50.0), vec2f(23.0, 23.0), 15.7));
    silhouette = max(silhouette, peak_land(q, vec2f(23.0 + shift, 61.0), vec2f(27.0, 32.0), 23.3));
    silhouette = max(silhouette, peak_land(q, vec2f(36.0, 40.0), vec2f(25.0, 32.0), 11.4));
    let back = smoothstep(13.0, 48.0, q.y);
    let ridge = 1.0 - abs(2.0 * noise(vec2f(q.x * 0.34 + q.y * 0.19, q.y * 0.19) + variant * 17.0) - 1.0);
    let shape = mix(-0.73, -0.16, back)
        + silhouette * mix(0.74, 1.19, ridge)
        + (fbm(q * 0.14 + variant * 15.0) - 0.49) * (1.2 + 4.6 * back);
    let flank = max(
        peak_land(q, vec2f(-20.0, 14.0), vec2f(15.4, 24.0), 4.1),
        peak_land(q, vec2f(18.0, 21.0), vec2f(13.2, 18.0), 2.7)
    );
    return shape + flank;
}

fn lake_trace(ro: vec3f, rd: vec3f, stop: f32, variant: f32, steps: u32) -> f32 {
    var distance = 0.12;
    var last = distance;
    for (var i = 0u; i < steps; i++) {
        let point = ro + rd * distance;
        let above = point.y - lake_land(point.xz, variant);
        if above < 0.0 {
            var lo = last;
            var hi = distance;
            for (var j = 0u; j < 5u; j++) {
                let mid = (lo + hi) * 0.5;
                let v = ro + mid * rd;
                if v.y > lake_land(v.xz, variant) { lo = mid; } else { hi = mid; }
            }
            return (lo + hi) * 0.5;
        }
        last = distance;
        distance += clamp(above * 0.58, 0.18, 2.75);
        if distance > stop { break; }
    }
    return -1.0;
}

fn lake_stone(point: vec3f, dist: f32, variant: f32, sun: vec3f) -> vec3f {
    let q = point.xz;
    let eps = 0.10 + dist * 0.001;
    let normal = normalize(vec3f(
        lake_land(q - vec2f(eps, 0.0), variant) - lake_land(q + vec2f(eps, 0.0), variant),
        2.0 * eps,
        lake_land(q - vec2f(0.0, eps), variant) - lake_land(q + vec2f(0.0, eps), variant)
    ));
    let pattern = fbm(q * 0.87 + point.y * 0.38 + variant * 20.0);
    let strata = noise(vec2f(point.y * 2.6 + point.x * 0.5, point.z * 0.53));
    let grain = noise(q * 6.6 + point.y * vec2f(0.75, 1.31));
    let detail = grain * (0.14 + p.detail * 0.23);
    let rock = mix(vec3f(0.13, 0.17, 0.21), vec3f(0.52, 0.49, 0.43), pattern * 0.66 + strata * 0.19 + detail);
    let snow_line = point.y + pattern * 5.4 + strata * 1.25;
    let cover = smoothstep(13.7, 17.9, snow_line) * smoothstep(0.45, 0.83, normal.y + pattern * 0.29);
    let snow = mix(vec3f(0.51, 0.66, 0.78), vec3f(0.95, 0.98, 1.0), pattern * 0.85 + 0.15);
    let trees = smoothstep(0.03, 0.46, point.y) * (1.0 - smoothstep(3.1, 8.1, point.y)) * smoothstep(0.28, 0.56, pattern);
    let canopy = noise(q * vec2f(10.5, 11.3) - point.y * vec2f(2.2, 5.0));
    let evergreen = mix(vec3f(0.034, 0.072, 0.052), vec3f(0.24, 0.34, 0.15), canopy * 0.67 + detail);
    let shore = mix(vec3f(0.24, 0.24, 0.19), vec3f(0.53, 0.46, 0.34), pattern);
    var base = mix(mix(rock, evergreen, trees), snow, cover);
    base = mix(shore, base, smoothstep(-0.04, 0.54, point.y));
    let diffuse = max(dot(normal, sun), 0.0);
    let shadow = lake_land((point + sun * 1.8).xz, variant) > point.y + sun.y * 1.8 + 0.17;
    let warm = mix(vec3f(1.34, 0.97, 0.71), vec3f(1.13, 1.12, 1.07), p.light);
    let lit = base * (vec3f(0.48, 0.60, 0.78) + warm * diffuse * select(0.98, 0.27, shadow));
    let haze = vec3f(0.40, 0.55, 0.69);
    let fog = 1.0 - exp(-dist * (0.0045 + 0.0035 * exp(-point.y * 0.47)));
    return mix(lit, haze, fog);
}

fn scene(film: vec2f) -> vec3f {
    let variant = seed_part(1u);
    let eye = vec3f(mix(-2.2, 2.2, p.camera), mix(1.36, 3.9, p.camera), -6.0);
    let rd = ray(eye, vec3f(0.8, 3.4, 33.0), film, 1.05);
    let sun = sunlight();
    let terrain_sun = normalize(vec3f(-0.71, mix(0.20, 0.99, p.light), -0.66));
    let sky = atmosphere(rd, sun, vec3f(0.07, 0.35, 0.57), variant * 37.0);
    let water_distance = select(125.0, min(-eye.y / min(rd.y, -0.0001), 125.0), rd.y < -0.0001);
    var land_distance = -1.0;
    if rd.y < 0.66 {
        land_distance = lake_trace(eye, rd, water_distance, variant, 104u);
    }
    if land_distance >= 0.0 {
        return lake_stone(eye + rd * land_distance, land_distance, variant, terrain_sun);
    }
    if water_distance >= 125.0 { return sky; }

    let point = eye + rd * water_distance;
    let q = point.xz;
    let waves = (noise(q * vec2f(2.7, 9.0)) - 0.5) * 0.040;
    let swells = (noise(q * vec2f(1.8, 4.4) + 6.0) - 0.5) * 0.043;
    let normal = normalize(vec3f(waves + swells * 0.35, 1.0, swells + waves * 0.15));
    let reflected = reflect(rd, normal);
    var reflection = atmosphere(reflected, sun, vec3f(0.07, 0.35, 0.57), variant * 37.0);
    if reflected.y < 0.61 {
        let bounce_origin = point + vec3f(0.0, 0.075, 0.0);
        let hit = lake_trace(bounce_origin, reflected, 100.0, variant, 72u);
        if hit >= 0.0 {
            reflection = lake_stone(bounce_origin + reflected * hit, hit + water_distance, variant, terrain_sun);
        }
    }
    let deep = max(0.0, -lake_land(q, variant));
    let stones = fbm(q * 3.1 + variant * 7.0) - 0.5;
    let bottom = mix(vec3f(0.17, 0.39, 0.33), vec3f(0.023, 0.22, 0.27), smoothstep(0.15, 1.0, deep));
    let fresnel = 0.28 + 0.67 * pow(1.0 - abs(dot(normal, -rd)), 3.0);
    let glint = pow(max(dot(reflected, sun), 0.0), 170.0) * (0.36 + 0.55 * noise(q * 7.0));
    let foam = (1.0 - smoothstep(0.005, 0.11, deep)) * 0.12 * smoothstep(0.52, 0.73, noise(q * 3.3));
    return mix(bottom + stones * 0.085, reflection * vec3f(0.73, 0.87, 0.93), fresnel)
        + glint * vec3f(1.0, 0.74, 0.39) + foam;
}
