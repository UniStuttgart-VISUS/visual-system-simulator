// https://www.w3.org/TR/2022/WD-WGSL-20220922/

// Vertex shader

struct Uniforms{
    flags: u32,
    proj_view: mat4x4<f32>,
};

@group(1) @binding(0)
var<uniform> uniforms: Uniforms;

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
};

struct FragmentOutput {
    @location(0) color: vec4<f32>,
    @location(1) deflection: vec4<f32>,
    @location(2) color_change: vec4<f32>,
    @location(3) color_uncertainty: vec4<f32>,
    @location(4) covariances: vec4<f32>,
};

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
    out.tex_coords = pos;
    out.clip_position = vec4<f32>(pos * 2.0 - 1.0, 0.0, 1.0);
    return out;
}

// Fragment shader

@group(0) @binding(0)
var in_color_t: texture_2d<f32>;
@group(0) @binding(1)
var in_color_s: sampler;

@fragment
fn fs_main(in: VertexOutput) -> FragmentOutput {
    var out: FragmentOutput;
    out.color = textureSample(in_color_t, in_color_s, in.tex_coords);
    // out.color = uniforms.test_color * vec4<f32>(in.tex_coords, 0.0, 1.0);
    return out;
}