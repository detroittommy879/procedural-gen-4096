struct Parameters {
    size: vec2f,
    seed: u32,
    samples: u32,
    light: f32,
    color: f32,
    camera: f32,
    detail: f32,
    time: f32,
    effects: u32,
    action: u32,
    progress: f32,
    transition: f32,
    frame_x: f32,
    frame_y: f32,
    zoom: f32,
}

@group(0) @binding(0) var<uniform> p: Parameters;

const TAU: f32 = 6.28318531;

const WF_CLOUDS: u32 = 1u;
const WF_REFLECTIONS: u32 = 2u;
const WF_PARTICLES: u32 = 4u;
const WF_SHADOWS: u32 = 8u;
const WF_GRAIN: u32 = 16u;
const WF_CAMERA: u32 = 32u;
const WF_ENSEMBLE: u32 = 64u;

fn wf_enabled(flag: u32) -> bool {
    return (p.effects & flag) != 0u;
}

fn hash(p0: vec2f) -> f32 {
    var q = fract(vec3f(p0.xyx) * 0.1031);
    q += dot(q, q.yzx + 33.33);
    return fract((q.x + q.y) * q.z);
}

fn seed_part(salt: u32) -> f32 {
    var n = p.seed ^ (salt * 0x9e3779b9u);
    n = (n ^ (n >> 16u)) * 0x7feb352du;
    n = (n ^ (n >> 15u)) * 0x846ca68bu;
    return f32((n ^ (n >> 16u)) & 0x00ffffffu) / 16777216.0;
}

fn noise(v: vec2f) -> f32 {
    let cell = floor(v);
    let f = fract(v);
    let t = f * f * (3.0 - 2.0 * f);
    return mix(
        mix(hash(cell), hash(cell + vec2f(1.0, 0.0)), t.x),
        mix(hash(cell + vec2f(0.0, 1.0)), hash(cell + vec2f(1.0, 1.0)), t.x),
        t.y
    );
}

fn fbm(v: vec2f) -> f32 {
    let rotation = mat2x2f(0.80, 0.60, -0.60, 0.80);
    let q = rotation * v * 2.03 + vec2f(14.0, 8.0);
    let r = rotation * q * 2.08 + vec2f(5.0, 13.0);
    return 0.55 * noise(v) + 0.29 * noise(q) + 0.16 * noise(r);
}

fn ray(eye: vec3f, look_at: vec3f, film: vec2f, lens: f32) -> vec3f {
    let forward = normalize(look_at - eye);
    let right = normalize(cross(vec3f(0.0, 1.0, 0.0), forward));
    let up = cross(forward, right);
    return normalize(forward * lens + right * film.x + up * film.y);
}

fn sunlight() -> vec3f {
    return normalize(vec3f(-0.55, mix(0.12, 0.95, p.light), 0.76));
}

fn atmosphere(rd: vec3f, sun: vec3f, tint: vec3f, offset: f32) -> vec3f {
    let elevation = max(rd.y, 0.0);
    let warm = 1.0 - smoothstep(0.12, 0.80, p.light);
    let horizon = mix(vec3f(0.65, 0.76, 0.79), vec3f(0.96, 0.49, 0.29), warm * 0.60);
    let zenith = mix(vec3f(0.055, 0.22, 0.49), tint, 0.34);
    var sky = mix(horizon, zenith, 1.0 - exp(-elevation * 4.2));
    let alignment = max(dot(rd, sun), 0.0);
    sky += vec3f(1.0, 0.50 + 0.25 * p.light, 0.22) * pow(alignment, 10.0) * (0.45 + warm * 0.23);
    sky += vec3f(1.0, 0.78, 0.51) * (0.25 * pow(alignment, 90.0) + 2.6 * smoothstep(0.99955, 0.99983, alignment));

    if !wf_enabled(WF_CLOUDS) { return sky; }
    let plane = rd.xz / (0.13 + elevation);
    let uv = plane * vec2f(2.1, 4.0) + vec2f(offset, offset * 0.77) + vec2f(p.time * 0.032, -p.time * 0.006);
    let warp = vec2f(noise(uv * 0.44 + 3.0), noise(uv * 0.44 + 14.0));
    let cloud = fbm(uv + warp * 1.5);
    let coverage = smoothstep(0.53, 0.73, cloud) * smoothstep(0.014, 0.10, elevation);
    let clear = fbm(uv * 0.47 + vec2f(7.2, 4.4));
    let bright = mix(vec3f(0.78, 0.85, 0.93), vec3f(1.0, 0.68, 0.47), warm * 0.65);
    let shade = mix(vec3f(0.36, 0.49, 0.67), vec3f(0.44, 0.34, 0.46), warm * 0.62);
    return mix(sky, mix(shade, bright, smoothstep(0.27, 0.75, clear + alignment * 0.18)), coverage * 0.87);
}

fn grade(rgb: vec3f, film: vec2f) -> vec3f {
    let luminance = dot(rgb, vec3f(0.2126, 0.7152, 0.0722));
    var color = mix(vec3f(luminance), rgb, mix(0.81, 1.42, p.color));
    color = max(color, vec3f(0.0));
    color *= 1.0 - 0.13 * smoothstep(0.21, 0.88, length(film));
    color = color * (1.0 + color * 0.24) / (1.0 + color * 0.43);
    return pow(clamp(color, vec3f(0.0), vec3f(1.0)), vec3f(0.88));
}

@vertex
fn fullscreen(@builtin(vertex_index) i: u32) -> @builtin(position) vec4f {
    let x = f32((i << 1u) & 2u);
    let y = f32(i & 2u);
    return vec4f(x * 2.0 - 1.0, y * 2.0 - 1.0, 0.0, 1.0);
}

@fragment
fn pixel(@builtin(position) position: vec4f) -> @location(0) vec4f {
    let film = (position.xy - 0.5 * p.size) * vec2f(1.0, -1.0) / min(p.size.x, p.size.y);
    let count = clamp(p.samples, 1u, 2u);
    var color = vec3f(0.0);
    for (var i = 0u; i < count; i++) {
        let jitter = select(0.0, 2.0 * f32(i) - 1.0, count == 2u);
        let offset = jitter * vec2f(0.38, -0.29) / min(p.size.x, p.size.y);
        color += wf_story_frame(film + offset);
    }
    color /= f32(count);
    let grain = select(0.0, (hash(position.xy + vec2f(seed_part(45u) * 19.0)) - 0.5) / 180.0, wf_enabled(WF_GRAIN));
    let fade = smoothstep(0.0, 1.0, p.transition);
    return vec4f((grade(color, film) + grain) * fade, 1.0);
}
