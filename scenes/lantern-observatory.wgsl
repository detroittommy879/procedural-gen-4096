// name: Lantern Observatory
// description: An old signal tower between a slate valley and a ringed moon.

fn obs_ridge(x: f32, phase: f32, ridge: f32) -> f32 {
    let range = fbm(vec2f(x * ridge, phase));
    let erosion = 1.0 - abs(2.0 * noise(vec2f(x * ridge * 2.7, phase + x * 0.08)) - 1.0);
    return range * 0.71 + erosion * 0.29;
}

fn obs_sky(film: vec2f, variant: f32, day: f32) -> vec3f {
    let band = smoothstep(-0.34, 0.59, film.y);
    let night_horizon = vec3f(0.33, 0.24, 0.38);
    let night_zenith = vec3f(0.028, 0.051, 0.16);
    let dawn_horizon = vec3f(0.89, 0.42, 0.28);
    let dawn_zenith = vec3f(0.11, 0.32, 0.54);
    var sky = mix(mix(night_horizon, night_zenith, band), mix(dawn_horizon, dawn_zenith, band), day);
    let q = film - vec2f(-0.28 + (variant - 0.5) * 0.21, 0.24);
    let disk = length(q) - 0.106;
    let moon_mask = 1.0 - smoothstep(-0.003, 0.002, disk);
    let sphere = sqrt(max(0.0, 1.0 - dot(q, q) / (0.106 * 0.106)));
    let phase = smoothstep(-0.25, 0.65, sphere * 0.30 - q.x * 8.0 + q.y * 4.0);
    let geology = fbm(q * vec2f(63.0, 110.0) + variant * 25.0);
    let moon = mix(vec3f(0.081, 0.106, 0.20), vec3f(0.87, 0.70, 0.57), phase * (0.71 + geology * 0.31));
    let orbit = vec2f(q.x * 0.93 + q.y * 0.37, q.y * 0.93 - q.x * 0.37);
    let ring_radius = length(orbit / vec2f(1.0, 0.22));
    let ring = smoothstep(0.113, 0.129, ring_radius) * (1.0 - smoothstep(0.178, 0.194, ring_radius));
    let ringing = ring * (0.29 + 0.52 * noise(vec2f(ring_radius * 208.0, variant * 16.0)));
    let ring_color = vec3f(0.68, 0.55, 0.49);
    if wf_enabled(WF_PARTICLES) {
        let grid = film * 142.0;
        let cell = floor(grid);
        let spot = vec2f(hash(cell + 16.0 + variant), hash(cell + 43.0 + variant)) * 0.76 + 0.12;
        let light = hash(cell + variant * 31.0);
        let star = exp(-length(fract(grid) - spot) * 18.0) * smoothstep(0.91, 0.997, light);
        let twinkle = 0.66 + 0.34 * sin(p.time * (0.44 + light) + light * 70.0);
        sky += vec3f(0.69, 0.78, 0.98) * star * twinkle * (1.0 - day * 0.92);
    }
    if wf_enabled(WF_CLOUDS) {
        let cloud_uv = film * vec2f(4.9, 31.0) + vec2f(p.time * 0.045, variant * 30.0);
        let vapor = smoothstep(0.49, 0.84, fbm(cloud_uv));
        let fade = smoothstep(-0.11, 0.02, film.y) * (1.0 - smoothstep(0.20, 0.47, film.y));
        sky = mix(sky, mix(vec3f(0.36, 0.28, 0.46), vec3f(0.95, 0.54, 0.35), day), vapor * fade * 0.52);
    }
    sky = mix(sky, ring_color, ringing * (1.0 - day * 0.67));
    sky = mix(sky, moon, moon_mask * (1.0 - day * 0.56));
    return mix(sky, ring_color, ringing * smoothstep(-0.004, 0.002, -orbit.y) * (1.0 - day * 0.67));
}

fn scene(film: vec2f) -> vec3f {
    let variant = seed_part(30u);
    let day = smoothstep(0.31, 0.78, p.light);
    let point = film + vec2f((p.camera - 0.5) * 0.17, 0.0);
    var color = obs_sky(point, variant, day);
    for (var i = 0; i < 4; i++) {
        let depth = f32(i);
        let phase = variant * 47.0 + depth * 11.3;
        let valley = smoothstep(0.02, 0.68, abs(point.x + (depth - 1.5) * 0.06));
        let height = -0.085 - depth * 0.040 + obs_ridge(point.x, phase, 3.8 + depth * 3.2) * (0.079 + valley * 0.225);
        let twilight = mix(vec3f(0.40, 0.31, 0.44), vec3f(0.62, 0.41, 0.42), day);
        let slate = mix(vec3f(0.047, 0.069, 0.118), vec3f(0.10, 0.20, 0.24), day);
        let surface = mix(twilight, slate, (depth + 1.0) / 4.0);
        let snowline = smoothstep(0.015, 0.0 + 0.085, height - point.y);
        let ridge_ink = surface * (0.89 + 0.12 * snowline + 0.15 * noise(point * 91.0 + phase));
        color = wf_ink(color, point.y - height, ridge_ink, 0.0018);
    }

    let ground = -0.265 - 0.053 * noise(vec2f(point.x * 12.0, variant * 17.0));
    let lake_depth = max(0.022, ground - point.y);
    let river = abs(point.x + 0.13 * sin(lake_depth * 8.1)) - (0.06 + 0.43 * lake_depth);
    let dirt = mix(vec3f(0.072, 0.095, 0.139), vec3f(0.22, 0.24, 0.23), day)
        * (0.72 + 0.51 * fbm(point * 55.0));
    var water = mix(vec3f(0.075, 0.113, 0.20), vec3f(0.29, 0.26, 0.31), day);
    if wf_enabled(WF_REFLECTIONS) {
        let streak = fbm(vec2f(point.x * 49.0, point.y * 142.0 - p.time * 0.33));
        let glint = exp(-abs(point.x - 0.06) * 19.0) * streak;
        water += mix(vec3f(0.21, 0.18, 0.27), vec3f(0.58, 0.32, 0.17), day) * glint;
    }
    color = wf_ink(color, point.y - ground, mix(dirt, water, 1.0 - smoothstep(-0.005, 0.013, river)), 0.0018);

    let tower = point - vec2f(0.13 + (variant - 0.5) * 0.19, -0.207);
    let hill = -0.032 + 0.035 * exp(-abs(tower.x) * 15.0) + 0.014 * fbm(point * 47.0);
    let foundation = mix(vec3f(0.067, 0.084, 0.12), vec3f(0.19, 0.21, 0.21), day);
    let base = max(tower.y - hill, -tower.y - 0.088 - abs(tower.x) * 0.083);
    color = wf_ink(color, max(base, abs(tower.x) - 0.19), foundation, 0.002);

    let stem = wf_box(tower, vec2f(0.0, 0.12), vec2f(0.033 + 0.028 * (1.0 - smoothstep(0.01, 0.26, tower.y)), 0.129), 0.007);
    let trim = vec3f(0.082, 0.11, 0.18) + day * vec3f(0.166, 0.12, 0.093);
    let panel = mix(trim * 0.60, trim * 1.52, smoothstep(-0.054, 0.027, tower.x));
    color = wf_ink(color, stem - 0.003, vec3f(0.054, 0.073, 0.11), 0.0018);
    color = wf_ink(color, stem, panel * (0.90 + 0.19 * noise(vec2f(tower.x * 214.0, tower.y * 92.0))), 0.0018);
    let deck = wf_box(tower, vec2f(0.0, 0.246), vec2f(0.090, 0.019), 0.008);
    let dome = max(wf_oval(tower, vec2f(0.0, 0.267), vec2f(0.067, 0.047)), 0.253 - tower.y);
    color = wf_ink(color, dome - 0.002, vec3f(0.095, 0.125, 0.21) + day * 0.17, 0.0021);
    color = wf_ink(color, deck, trim * 0.88, 0.0021);
    let ribs = abs(fract((tower.x + 0.06) * 62.0) - 0.5);
    let windows = wf_box(tower, vec2f(0.0, 0.258), vec2f(0.071, 0.009), 0.004);
    let power = select(1.0, 0.09, p.action == 1u);
    let startup = select(power, smoothstep(0.53, 0.60, p.progress), p.action == 7u);
    color = wf_ink(color, max(windows, 0.15 - ribs), vec3f(0.19, 0.23, 0.33) + startup * vec3f(0.73, 0.44, 0.14), 0.002);
    let antenna = wf_box(tower, vec2f(0.0, 0.317), vec2f(0.004, 0.026), 0.001);
    color = wf_ink(color, antenna, trim * 1.40, 0.0018);

    let tip = tower - vec2f(0.0, 0.344);
    let turn = cos(p.time * 0.81 + 1.2);
    let beam = exp(-abs(tip.y - tip.x * turn * 0.23) * (76.0 + p.detail * 42.0)) * exp(-abs(tip.x) * 2.8);
    let sweep = smoothstep(-0.014, 0.07, tip.x * sin(p.time * 0.81 + 1.2));
    let amber = vec3f(1.0, 0.53, 0.18);
    color += startup * amber * (0.18 * beam * sweep + 0.48 * exp(-length(tip) * 29.0));
    return wf_ink(color, length(tip) - 0.006, vec3f(0.33, 0.26, 0.18) + startup * amber, 0.002);
}
