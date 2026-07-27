struct Meta { rotation: f32, full_range: f32, has_depth: f32, _pad0: f32 }
@group(0) @binding(0) var y_tex: texture_2d<f32>;
@group(0) @binding(1) var uv_tex: texture_2d<f32>;
@group(0) @binding(2) var depth_tex: texture_2d<f32>;
@group(0) @binding(3) var samp: sampler;
@group(0) @binding(4) var<uniform> frame_meta: Meta;
struct Out { @builtin(position) pos: vec4<f32>, @location(0) uv: vec2<f32> }
struct FragmentOutput { @builtin(frag_depth) depth: f32, @location(0) color: vec4<f32> }
@vertex fn vs_main(@builtin(vertex_index) i: u32) -> Out {
    var p = array<vec2<f32>, 3>(vec2(-1.0,-1.0), vec2(3.0,-1.0), vec2(-1.0,3.0)); var o: Out; o.pos=vec4(p[i],0.0,1.0); o.uv=p[i]*vec2(0.5,-0.5)+vec2(0.5); return o;
}
fn rotated(uv: vec2<f32>) -> vec2<f32> { let r=i32(frame_meta.rotation); if r==90 { return vec2(uv.y,1.0-uv.x); } if r==180 { return 1.0-uv; } if r==270 { return vec2(1.0-uv.y,uv.x); } return uv; }
@fragment fn fs_main(i: Out) -> FragmentOutput {
    let uv=rotated(i.uv); var y=textureSample(y_tex,samp,uv).r; let c=textureSample(uv_tex,samp,uv).rg-vec2(0.5);
    if frame_meta.full_range < 0.5 { y=(y-16.0/255.0)*(255.0/219.0); }
    var out: FragmentOutput;
    out.color=vec4(y+1.5748*c.y, y-0.1873*c.x-0.4681*c.y, y+1.8556*c.x, 1.0);
    out.depth=0.0;
    if frame_meta.has_depth > 0.5 {
        let depth_m=textureSample(depth_tex,samp,uv).r;
        if depth_m > 0.0 && depth_m < 1000.0 {
            out.depth=clamp((5.0-depth_m)/(5.0-0.2),0.0,1.0);
        }
    }
    return out;
}
