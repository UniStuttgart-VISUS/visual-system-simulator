struct Uniforms{
    viewport: vec4<f32>,
    resolution_in: vec2<f32>,
    resolution_out: vec2<f32>,

    output_scale: u32,
    absolute_viewport: u32,
    _padding1: u32,
    _padding2: u32,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
};

struct FragmentOutput {
    @location(0) color: vec4<f32>,
};

@group(0) @binding(0)
var<uniform> uniforms: Uniforms;

@group(1) @binding(0)
var in_color_t: texture_2d<f32>;
@group(1) @binding(1)
var in_color_s: sampler;

@vertex
fn vs_main(
    @builtin(vertex_index) in_vertex_index: u32,
) -> VertexOutput {
    var out: VertexOutput;
    // create two triangles with edges (0,0),(1,0),(1,1) and (1,1),(0,1),(0,0)
    var x = f32((i32(in_vertex_index) % 3) > 0);
    var y = f32((i32(in_vertex_index) % 3) > 1);
    if((i32(in_vertex_index) / 3) > 0){
        x = 1.0-x;
        y = 1.0-y;
    }
    var pos = vec2<f32>(x, y);
    var tex = vec2<f32>(x, y);

    var view = uniforms.viewport;
    if(uniforms.absolute_viewport == 1u){ // ensure relative viewport with coordinates from 0.0 to 1.0
        view = vec4<f32>(view.xy / uniforms.resolution_out, view.zw / uniforms.resolution_out);
    }
    let ratio = (uniforms.resolution_out * view.zw) / uniforms.resolution_in;

    if(uniforms.output_scale == 0u){ // Fit
        pos = (pos - 0.5) * (min(ratio.x, ratio.y) / ratio) + 0.5;
    }else if(uniforms.output_scale == 1u){ // Fill
        tex = (tex - 0.5) / (max(ratio.x, ratio.y) / ratio) + 0.5;
    }// Stretch: no adjustment needed

    pos = pos * view.zw + view.xy;

    out.tex_coords = vec2<f32>(tex.x, 1.0 - tex.y);
    out.clip_position = vec4<f32>(pos * 2.0 - 1.0, 0.0, 1.0);
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> FragmentOutput {
    var out: FragmentOutput;

    out.color = textureSampleLevel(in_color_t, in_color_s, in.tex_coords, 0.0);
    return out;
}