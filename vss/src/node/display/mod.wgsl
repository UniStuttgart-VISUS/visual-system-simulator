// Vertex shader

struct Uniforms{
    stereo: i32,
    resolution_in: vec2<f32>,
    resolution_out: vec2<f32>,
    flow_idx: i32,

    heat_scale: f32,
    dir_calc_scale: f32,

    hive_rotation: mat4x4<f32>,
    hive_position: vec2<f32>,
    hive_visible: i32,

    base_image: i32,
    combination_function: i32,
    mix_type: i32,
    colormap_type: i32,
};

@group(0) @binding(0)
var<uniform> uniforms: Uniforms;

struct VertexOutput {
    @builtin(position) pos: vec4<f32>,
    @location(0) tex: vec2<f32>,
};

struct FragmentOutput {
    @location(0) color: vec4<f32>,
};

@vertex
fn vs_main(
    @builtin(vertex_index) in_vertex_index: u32,
) -> VertexOutput {
    var out: VertexOutput;
    let ratio = uniforms.resolution_out / uniforms.resolution_in;
    let scale = ratio / min(ratio.x, ratio.y);
    // for every 6 vertices create two triangles with edges (0,0),(1,0),(1,1) and (1,1),(0,1),(0,0)
    var x = f32((i32(in_vertex_index) % 3) > 0);
    var y = f32((i32(in_vertex_index) % 3) > 1);
    if(((i32(in_vertex_index) / 3) % 2) == 1){
        x = 1.0-x;
        y = 1.0-y;
    }
    out.tex = (vec2<f32>(x, y) - 0.5) * scale + 0.5;
    out.pos = vec4<f32>(vec2<f32>(x, y) * 2.0 - 1.0, 0.0, 1.0);
    if (uniforms.stereo == 1) {
        // shift x position to right or left half of screen
        out.pos.x /= 2.0;
        if (i32(in_vertex_index) < 6) {
            out.pos.x += 0.5;
        } else {
            out.pos.x -= 0.5;
        }
    }
    return out;
}

// Fragment shader

@group(1) @binding(0)
var in_sampler: sampler;
@group(1) @binding(1)
var in_color: texture_2d<f32>;
@group(1) @binding(2)
var in_deflection: texture_2d<f32>;
@group(1) @binding(3)
var in_color_change: texture_2d<f32>;
@group(1) @binding(4)
var in_color_uncertainty: texture_2d<f32>;
@group(1) @binding(5)
var in_covariances: texture_2d<f32>;

@group(2) @binding(0)
var in_original_t: texture_2d<f32>;
@group(2) @binding(1)
var in_original_s: sampler;

@fragment
fn fs_main(in: VertexOutput) -> FragmentOutput {
    var out: FragmentOutput;
    out.color = textureSample(in_color, in_sampler, in.tex);
    // out.color = uniforms.test_color * vec4<f32>(in.tex_coords, 0.0, 1.0);
    return out;
}