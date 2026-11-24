// name: Tidal Engine
// description: A brass tide computer turns inside a flooded turbine hall.

fn engine_rotate(q: vec2f, a: f32) -> vec2f {
    let c = cos(a);
    let s = sin(a);
    return vec2f(c*q.x - s*q.y, s*q.x + c*q.y);
}
fn engine_box(q: vec3f, bounds: vec3f) -> f32 {
    let d = abs(q) - bounds;
    return length(max(d, vec3f(0.0))) + min(max(d.x, max(d.y, d.z)), 0.0);
}
fn engine_nearest(a: vec2f, d: f32, material: f32) -> vec2f {
    return select(a, vec2f(d, material), d < a.x);
}
fn engine_map(q: vec3f, t: f32, variant: f32) -> vec2f {
    // Water, masonry and repeating steel roof ribs bound the interior.
    var hit = vec2f(q.y, 0.0);
    hit = engine_nearest(hit, min(8.4-abs(q.x), min(7.8-q.y, 21.0-q.z)), 1.0);
    let z = fract((q.z+2.0)/4.0)*4.0-2.0;
    let post = engine_box(vec3f(abs(q.x)-6.8,q.y-3.0,z), vec3f(0.16,3.4,0.19));
    let beam = max(abs(z)-0.16, abs(length(vec2f(q.x,(q.y-1.8)*1.14))-6.94)*0.82-0.12);
    hit = engine_nearest(hit, min(post, beam), 2.0);
    let rail = engine_box(vec3f(abs(q.x)-5.9,q.y-1.08,q.z-5.0), vec3f(1.1,0.13,18.0));
    hit = engine_nearest(hit, rail, 2.0);
    let handrail=length(vec2f(abs(q.x)-5.0,q.y-1.91))-0.046;
    let upright=engine_box(vec3f(abs(q.x)-5.0,q.y-1.53,z),vec3f(0.048,0.39,0.048));
    hit=engine_nearest(hit,min(handrail,upright),3.0);
    // Supply pipes run along both walls, with thicker collars at each vault.
    let pipes=length(vec2f(abs(q.x)-7.52,abs(q.y-3.1)-0.33))-0.20;
    let collar=max(pipes-0.083,abs(z)-0.13);
    hit=engine_nearest(hit,min(pipes,collar),3.0);
    // A pair of extraction fans draws air through the far bulkhead.
    let vent=vec2f(abs(q.x)-3.15,q.y-4.3);
    let rim=length(vec2f(length(vent)-1.72,q.z-20.62))-0.105;
    hit=engine_nearest(hit,rim,3.0);
    if q.z>18.0 && length(vent)<1.75 {
        let spin=atan2(vent.y,vent.x)+t*0.42+length(vent)*0.31;
        let vane=max(abs(sin(spin*5.0))*length(vent)-0.19,length(vent)-1.61);
        hit=engine_nearest(hit,max(vane,abs(q.z-20.67)-0.062),2.0);
        hit=engine_nearest(hit,length(vec3f(vent,q.z-20.61))-0.36,3.0);
    }
    let lamp = engine_box(vec3f(abs(q.x)-4.86,q.y-1.13,q.z-5.0), vec3f(0.065,0.032,17.8));
    hit = engine_nearest(hit, lamp, 4.0);
    let v = q - vec3f(0.0,2.75,2.4);
    let radial = length(v.xz);
    hit = engine_nearest(hit, max(radial-1.52,abs(q.y-0.21)-0.23), 2.0);
    hit = engine_nearest(hit, max(radial-0.69,abs(q.y-0.59)-0.30), 3.0);
    let bracket=engine_box(vec3f(abs(v.x)-1.24,q.y-0.74,v.z),vec3f(0.095,0.51,0.16));
    hit=engine_nearest(hit,bracket,2.0);
    if length(v) > 3.25 { return engine_nearest(hit, length(v)-2.90, 1.0); }
    // Three independently geared gimbals carry engraved rims and brass counterweights.
    for(var i=0; i<3; i++) {
        let k=f32(i);
        let xy=engine_rotate(v.xy,0.39+k*1.04+variant*0.42);
        let yz=engine_rotate(vec2f(xy.y,v.z),t*(0.17+k*0.043)+k*0.96);
        let u=vec3f(engine_rotate(vec2f(xy.x,yz.x),t*(0.12+k*0.06)),yz.y);
        let radius=2.13-k*0.40;
        let angle=atan2(u.y,u.x);
        let tooth=smoothstep(-0.4,0.55,cos(angle*(32.0+k*8.0)));
        let rim=max(abs(length(u.xy)-radius)-0.072-tooth*0.042, abs(u.z)-0.072);
        hit=engine_nearest(hit,rim,3.0);
        let tracer=max(abs(length(u.xy)-radius+0.020)-0.018,abs(abs(u.z)-0.080)-0.014);
        hit=engine_nearest(hit,tracer,4.0);
        let spoke=max(abs(abs(u.x)-radius*0.51)-radius*0.49,length(u.yz)-0.037);
        hit=engine_nearest(hit,spoke,2.0);
        hit=engine_nearest(hit,length(u-vec3f(radius,0.0,0.0))-0.165,3.0);
    }
    let ball = length(v);
    let lat = abs(sin(atan2(v.y,length(v.xz))*9.0));
    let lon = abs(sin((atan2(v.z,v.x)+t*0.38)*7.0));
    return engine_nearest(hit,ball-0.65,select(5.0,3.0,min(lat,lon)<0.115));
}
fn engine_normal(q:vec3f,t:f32,v:f32) -> vec3f {
    let e=vec2f(0.003,0.0);
    let d=engine_map(q,t,v).x;
    return normalize(vec3f(engine_map(q+e.xyy,t,v).x,engine_map(q+e.yxy,t,v).x,engine_map(q+e.yyx,t,v).x)-d);
}
fn engine_trace(ro:vec3f, rd:vec3f,t:f32,v:f32,steps:u32) -> vec3f {
    var travel=0.025;
    for(var i=0u;i<steps;i++) {
        let probe=engine_map(ro+rd*travel,t,v);
        if probe.x<0.002+travel*0.00017 {return vec3f(travel,probe.y,1.0);}
        travel+=clamp(probe.x*0.88,0.005,2.4);
        if travel>36.0 {break;}
    }
    return vec3f(travel,0.0,0.0);
}
fn engine_shade(q:vec3f, rd:vec3f, mat:f32,t:f32,v:f32,power:f32) -> vec3f {
    let n=engine_normal(q,t,v);
    let lamp=vec3f(-2.0,5.6,-1.0);
    let l=normalize(lamp-q);
    let alloy=mix(vec3f(0.36,0.16,0.058),vec3f(0.90,0.59,0.24),noise(q.xz*28.0+q.y*17.0)*0.32+0.45);
    let plate=abs(fract(q.zy*vec2f(0.48,0.7)+q.x*0.27)-0.5);
    let seam=smoothstep(0.005,0.040,min(plate.x,plate.y));
    var base=mix(vec3f(0.053,0.105,0.15),vec3f(0.17,0.26,0.28),seam*fbm(q.xz*2.8+q.y));
    if mat==2.0 {base=vec3f(0.18,0.24,0.27)*mix(0.81,1.1,seam);}
    if mat==3.0 {base=alloy;}
    if mat==4.0 {return vec3f(0.17,0.96,0.87)*(0.4+power);}
    if mat==5.0 {return vec3f(1.18,0.55,0.16)*(power+0.25)*(0.79+0.21*cos(q.y*21.0-t*2.0));}
    var ao=1.0;
    if wf_enabled(WF_SHADOWS) {
        ao=clamp(engine_map(q+n*0.24,t,v).x/0.24,0.30,1.0);
    }
    let metal=pow(max(dot(n,normalize(l-rd)),0.0),mix(18.0,72.0,p.detail));
    let bounce=normalize(vec3f(2.4,2.5,4.0)-q);
    return base*(vec3f(0.27,0.39,0.48)*ao+max(dot(n,l),0.0)*vec3f(1.17,0.87,0.62)
        +max(dot(n,bounce),0.0)*vec3f(0.13,0.34,0.39))+metal*vec3f(0.94,0.83,0.60);
}
fn scene(film:vec2f) -> vec3f {
    let v=seed_part(71u);
    let power=select(0.80,0.12+0.88*smoothstep(0.26,0.67,p.progress),p.action==8u);
    // Integrating a smooth speed ramp keeps rotor phase continuous on seek.
    let t=select(p.time,p.time*0.14+max(0.0,p.progress-0.27)*max(0.0,p.progress-0.27)*110.0,p.action==8u);
    let az=(p.camera-0.5)*1.12+0.105*sin(p.time*0.068+v);
    let eye=vec3f(7.6*sin(az),1.89+0.23*p.light,2.4-7.6*cos(az));
    let rd=ray(eye,vec3f(0.0,2.57,2.4),film,1.10);
    let hit=engine_trace(eye,rd,t,v,88u);
    let mist=vec3f(0.062,0.129,0.16);
    var rgb=mist;
    if hit.z>0.0 {
        let q=eye+rd*hit.x;
        if hit.y==0.0 {
            rgb=vec3f(0.026,0.110,0.122);
            if wf_enabled(WF_REFLECTIONS) {
                let rip=vec2f(noise(q.xz*vec2f(4.0,8.0)+p.time*0.51),noise(q.xz*7.0-p.time*0.33))-0.5;
                let refl=reflect(rd,normalize(vec3f(rip.x*0.077,1.0,rip.y*0.08)));
                let origin=q+vec3f(0.0,0.025,0.0);
                let h=engine_trace(origin,refl,t,v,56u);
                if h.z>0.0 && h.y>0.0 {rgb+=0.56*engine_shade(origin+refl*h.x,refl,h.y,t,v,power);}
            }
        } else {rgb=engine_shade(q,rd,hit.y,t,v,power);}
        rgb=mix(rgb,mist,1.0-exp(-hit.x*0.018));
    }
    let core=vec3f(0.0,2.75,2.4)-eye;
    let closest=clamp(dot(core,rd),0.0,hit.x);
    rgb+=vec3f(0.90,0.37,0.10)*power*0.30*exp(-length(core-rd*closest)*1.7);
    return rgb*mix(0.80,1.25,p.light);
}
