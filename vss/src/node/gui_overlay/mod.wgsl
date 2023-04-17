struct Uniforms{
    resolution_in: vec2<f32>,
    resolution_out: vec2<f32>,
}

struct FragmentOutput {
    @location(0) color: vec4<f32>,
};

@group(0) @binding(0)
var<uniform> uniforms: Uniforms;

@group(1) @binding(0)
var in_color_t: texture_2d<f32>;
@group(1) @binding(1)
var in_color_s: sampler;

@fragment
fn fs_main(in: VertexOutput) -> FragmentOutput {
    var out: FragmentOutput;

    out.color = textureSampleLevel(in_color_t, in_color_s, in.tex_coords, 0.0);
    return out;
}