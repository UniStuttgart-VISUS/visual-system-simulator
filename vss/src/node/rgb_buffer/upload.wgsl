// https://www.w3.org/TR/2022/WD-WGSL-20220922/

struct Uniforms{
    flags: u32,
    proj_view: mat4x4<f32>,
};

@group(1) @binding(0)
var<uniform> uniforms: Uniforms;

struct FragmentOutput {
    @location(0) color: vec4<f32>,
    @location(1) deflection: vec4<f32>,
    @location(2) color_change: vec4<f32>,
    @location(3) color_uncertainty: vec4<f32>,
    @location(4) covariances: vec4<f32>,
};

// Fragment shader

@group(0) @binding(0)
var in_color_t: texture_2d<f32>;
@group(0) @binding(1)
var in_color_s: sampler;

@fragment
fn fs_main(in: VertexOutput) -> FragmentOutput {
    var out: FragmentOutput;
    out.color = textureSample(in_color_t, in_color_s, in.tex_coords);
    out.deflection = vec4<f32>(in.tex_coords, 0.0f, 1.0f);
    // out.color = uniforms.test_color * vec4<f32>(in.tex_coords, 0.0, 1.0);
    return out;
}