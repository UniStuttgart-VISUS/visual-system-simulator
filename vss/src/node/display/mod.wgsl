struct Uniforms{
    resolution_in: vec2<f32>,
    resolution_out: vec2<f32>,

    stereo: i32,
    flow_idx: i32,
};

struct VertexOutput {
    @builtin(position) pos: vec4<f32>,
    @location(0) tex: vec2<f32>,
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
    let ratio = uniforms.resolution_out / uniforms.resolution_in;
    let scale = ratio / min(ratio.x, ratio.y);
    // for every 6 vertices create two triangles with edges (0,0),(1,0),(1,1) and (1,1),(0,1),(0,0)
    var x = f32((i32(in_vertex_index) % 3) > 0);
    var y = f32((i32(in_vertex_index) % 3) > 1);
    if(((i32(in_vertex_index) % 6) / 3) > 0){
        x = 1.0-x;
        y = 1.0-y;
    }
    out.tex = (vec2<f32>(x, 1.0 - y) - 0.5) * scale + 0.5;
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

@fragment
fn fs_main(in: VertexOutput) -> FragmentOutput {
    var out: FragmentOutput;

    if (in.tex.x < 0.0 || in.tex.y < 0.0 ||
        in.tex.x > 1.0 || in.tex.y > 1.0) {
        discard;
    }

    out.color = textureSampleLevel(in_color_t, in_color_s, in.tex, 0.0);
    return out;
}