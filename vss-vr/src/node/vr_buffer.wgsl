// https://www.w3.org/TR/2022/WD-WGSL-20220922/

struct FragmentOutput {
    @builtin(frag_depth) depth: f32,
    @location(0) color: vec4<f32>,
    @location(1) deflection: vec4<f32>,
    @location(2) color_change: vec4<f32>,
    @location(3) color_uncertainty: vec4<f32>,
    @location(4) covariances: vec4<f32>,
};

// Fragment shader

@group(0) @binding(0)
var in_color_s: sampler;
@group(0) @binding(1)
var in_color_t: texture_2d<f32>;
@group(0) @binding(2)
var in_depth_s: sampler;
@group(0) @binding(3)
var in_depth_t: texture_2d<f32>;

@fragment
fn fs_main(in: VertexOutput) -> FragmentOutput {
    var out: FragmentOutput;

    out.color = vec4<f32>(textureSample(in_color_t, in_color_s, in.tex_coords).rgb, 1.0);
    out.depth = textureSample(in_depth_t, in_depth_s, in.tex_coords).r;

    out.deflection =         vec4<f32>(0.0);
    out.color_change =       vec4<f32>(0.0);
    out.color_uncertainty =  vec4<f32>(0.0);
    out.covariances =        vec4<f32>(0.0);

    return out;
}
