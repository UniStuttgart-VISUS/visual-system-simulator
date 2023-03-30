
struct Uniforms{
    resolution_in: vec2<f32>,
    resolution_out: vec2<f32>,
    flow_idx: i32,
    _padding: u32,
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
var in_color_r_t: texture_2d<f32>;
@group(1) @binding(1)
var in_color_r_s: sampler;

@group(2) @binding(0)
var in_color_l_t: texture_2d<f32>;
@group(2) @binding(1)
var in_color_l_s: sampler;

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
    let pos = vec2<f32>(x, y);
    let ratio = uniforms.resolution_out / uniforms.resolution_in;
    let scale = ratio / min(ratio.x, ratio.y);

    out.tex_coords = vec2<f32>(pos.x, 1.0 - pos.y);
    out.tex_coords = scale * (out.tex_coords - 0.5) + 0.5;
    out.clip_position = vec4<f32>(pos * 2.0 - 1.0, 0.0, 1.0);
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> FragmentOutput {
    var out: FragmentOutput;

    let right_color =  textureSample(in_color_r_t, in_color_r_s, in.tex_coords);
    let left_color =  textureSample(in_color_l_t, in_color_l_s, in.tex_coords);
    out.color = select(left_color, right_color, uniforms.flow_idx == 0);

    if(in.tex_coords.x < 0.0 || in.tex_coords.y < 0.0 ||
        in.tex_coords.x > 1.0 || in.tex_coords.y > 1.0){
        discard;
    }

    return out;
}