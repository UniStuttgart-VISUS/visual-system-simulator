// #include "common.wgsl"

struct Uniforms{
    resolution: vec2<f32>,
    blur_factor: f32,
    contrast_factor: f32,
    is_active: i32,
    track_error: i32,
}

@group(0) @binding(0)
var<uniform> uniforms: Uniforms;

struct FragmentOutput {
    @location(0) color: vec4<f32>,
    @location(1) deflection: vec4<f32>,
    @location(2) color_change: vec4<f32>,
    @location(3) color_uncertainty: vec4<f32>,
    @location(4) covariances: vec4<f32>,
};

fn lowerContrastBy(color: vec4<f32>, S: ptr<function, mat3x3<f32>>, value: f32) -> vec4<f32>{
    var newColor = color;
    if (color.r > color.g && color.r > color.b) {
        newColor.g += (color.r - color.g) * value;
        newColor.b += (color.r - color.b) * value;
        // jacobian ([ r, g+(r-g)*c, b+(r-b)*c ], [r,g,b]);
        let J = mat3x3<f32>(
            vec3<f32>(1.0, 0.0, 0.0),
            vec3<f32>(value, 1.0-value, 0.0),
            vec3<f32>(value, 0.0, 1.0-value));
        *S = J*(*S)*transpose(J);
    } else if (color.g > color.r && color.g > color.b) {
        newColor.r += (color.g - color.r) * value;
        newColor.b += (color.g - color.b) * value;
        let J = transpose(mat3x3<f32>(
            vec3<f32>(1.0-value, value, 0.0),
            vec3<f32>(0.0, 1.0, 0.0),
            vec3<f32>(0.0, value, 1.0-value)));
        *S = J*(*S)*transpose(J);
    } else{
        newColor.r += (color.b - color.r) * value;
        newColor.g += (color.b - color.g) * value;
        let J = transpose(mat3x3<f32>(
            vec3<f32>(1.0-value, 0.0, value),
            vec3<f32>(0.0, 1.0-value, value),
            vec3<f32>(0.0, 0.0, 1.0)));
        *S = J*(*S)*transpose(J);
    }
    return newColor;
}

// Fragment shader

@group(1) @binding(0)
var in_color_s: sampler;
@group(1) @binding(1)
var in_color_t: texture_2d<f32>;
@group(1) @binding(2)
var in_deflection_s: sampler;
@group(1) @binding(3)
var in_deflection_t: texture_2d<f32>;
@group(1) @binding(4)
var in_color_change_s: sampler;
@group(1) @binding(5)
var in_color_change_t: texture_2d<f32>;
@group(1) @binding(6)
var in_color_uncertainty_s: sampler;
@group(1) @binding(7)
var in_color_uncertainty_t: texture_2d<f32>;
@group(1) @binding(8)
var in_covariances_s: sampler;
@group(1) @binding(9)
var in_covariances_t: texture_2d<f32>;

@fragment
fn fs_main(in: VertexOutput) -> FragmentOutput {
    var out: FragmentOutput;
    if (1 == uniforms.is_active) {

        var color: vec4<f32>;

        if( uniforms.track_error == 1 ){
            let original_color = textureSample(in_color_t, in_color_s, in.tex_coords).rgb;
            var color_var = textureSample(in_color_uncertainty_t, in_color_uncertainty_s, in.tex_coords).rgb;
            var color_covar = textureSample(in_covariances_t, in_covariances_s, in.tex_coords).rgb;
            var dir_var = textureSample(in_deflection_t, in_deflection_s, in.tex_coords).ba;
            var dir_covar = textureSample(in_covariances_t, in_covariances_s, in.tex_coords).a;
            
            var S_col = covarMatFromVec3(color_var, color_covar);
            var S_pos = covarMatFromVec2(dir_var, dir_covar);

            // the 3.0 is used to strengthen the blur compared to the bloom effect
            color =  blur_with_error(in.tex_coords, in_color_s, in_color_t, uniforms.blur_factor * 3.0, uniforms.resolution, &S_col, &S_pos, in_color_uncertainty_s, in_color_uncertainty_t, in_covariances_s, in_covariances_t, in_deflection_s, in_deflection_t);

            color = vec4<f32>(applyBloom(color.rgb, uniforms.blur_factor/3.0, &S_col), color.a);
            color = lowerContrastBy(color, &S_col, uniforms.contrast_factor);
            out.color = color;

            //write back
            covarMatToVec3(S_col, &color_var, &color_covar);
            covarMatToVec2(S_pos, &dir_var, &dir_covar);

            // update the rgb values of the textures with the new data. do not touch the alpha
            out.color_change = vec4<f32>(textureSample(in_color_change_t, in_color_change_s, in.tex_coords).rgb + ( out.color.rgb - original_color), 0.0);
            out.color_uncertainty = vec4<f32>( color_var, 0.0 );
            out.deflection = vec4<f32>(textureSample(in_deflection_t, in_deflection_s, in.tex_coords).rg, dir_var);

            out.covariances = vec4<f32>(color_covar, dir_covar);
        }else{
            color =  blur(in.tex_coords, in_color_s, in_color_t, uniforms.blur_factor * 3.0, uniforms.resolution);

            // since bloom and contrast are quite cheap, they do not have their own methods without error tracking
            var unused_mat = mat3x3<f32>();
            color = vec4<f32>(applyBloom(color.rgb, uniforms.blur_factor/3.0, &unused_mat), color.a);
            out.color = lowerContrastBy(color, &unused_mat, uniforms.contrast_factor);
        }

    }else{
        out.color = textureSample(in_color_t, in_color_s, in.tex_coords);

        if ( uniforms.track_error == 1 ){
            out.color_change =      textureSample(in_color_change_t, in_color_change_s, in.tex_coords);
            out.color_uncertainty = textureSample(in_color_uncertainty_t, in_color_uncertainty_s, in.tex_coords);
            out.deflection =        textureSample(in_deflection_t, in_deflection_s, in.tex_coords);
            out.covariances =       textureSample(in_covariances_t, in_covariances_s, in.tex_coords);
        }
    }
    return out;
    // out.depth = vec4<f32>(texture(s_depth, in.tex_coords)).r;
}
