// https://www.w3.org/TR/2022/WD-WGSL-20220922/

const PI: f32 = 3.14159265359;

struct Uniforms{
    inv_proj_view: mat4x4<f32>,
    flags: u32,
};

@group(1) @binding(0)
var<uniform> uniforms: Uniforms;

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
var in_color_t: texture_2d<f32>;
@group(0) @binding(1)
var in_color_s: sampler;

@fragment
fn fs_main(in: VertexOutput) -> FragmentOutput {
    var out: FragmentOutput;

    var depth = 0.5;
    var tex = in.tex_coords;
         
    if ((uniforms.flags & 1u) == 1u) {
        // Equirectangular 360° projection.
        let ndc = vec4<f32>(vec2(tex.x, 1.0 - tex.y) * 2.0 - 1.0, 0.9, 1.0);
        var world_dir = uniforms.inv_proj_view * ndc;
        world_dir = vec4<f32>(normalize(world_dir.xyz)/world_dir.w, world_dir.w);
        tex = vec2<f32>(atan2(world_dir.z, world_dir.x) + PI, acos(world_dir.y)) / vec2<f32>(2.0 * PI, PI);
    }

    if ((uniforms.flags & 2u) == 2u) {
        // Vertically flipped.
        tex = vec2<f32>(tex.x, 1.0 - tex.y);
    }

    if ((uniforms.flags & 4u) == 4u) {
        // RGBD horizontally splitted.
        let tex_depth = vec2<f32>(tex.x, 0.5 * tex.y);
        tex = vec2<f32>(tex.x, 0.5 * tex.y + 0.5);
        depth = textureSample(in_color_t, in_color_s, tex_depth).r;
    }

    out.color = vec4<f32>(textureSample(in_color_t, in_color_s, tex).rgb, 1.0);
    out.depth = depth;

    out.deflection =         vec4<f32>(0.0);
    out.color_change =       vec4<f32>(0.0);
    out.color_uncertainty =  vec4<f32>(0.0);
    out.covariances =        vec4<f32>(0.0);

    return out;
}
