// #include "common.wgsl"

struct Uniforms{
    resolution: vec2<f32>,
    blur_factor: f32,
    contrast_factor: f32,
    is_active: i32,
    track_error: i32,
    _padding: vec2<i32>,
}

@group(0) @binding(0)
var<uniform> uniforms: Uniforms;

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

struct SimulationOutput {
    depth: f32,
    color: vec4<f32>,
    deflection: vec4<f32>,
    color_change: vec4<f32>,
    color_uncertainty: vec4<f32>,
    covariances: vec4<f32>,
};

struct ColorOutput {
    @builtin(frag_depth) depth: f32,
    @location(0) color: vec4<f32>,
};

struct MetricsAbOutput {
    @location(0) metrics_a: vec4<f32>,
    @location(1) metrics_b: vec4<f32>,
};

struct MetricsCdOutput {
    @location(0) metrics_c: vec4<f32>,
    @location(1) metrics_d: vec4<f32>,
};

@group(1) @binding(0)
var in_color_s: sampler;
@group(1) @binding(1)
var in_color_t: texture_2d<f32>;
@group(1) @binding(2)
var in_depth_s: sampler;
@group(1) @binding(3)
var in_depth_t: texture_2d<f32>;
@group(1) @binding(4)
var in_metrics_a_s: sampler;
@group(1) @binding(5)
var in_metrics_a_t: texture_2d<f32>;
@group(1) @binding(6)
var in_metrics_b_s: sampler;
@group(1) @binding(7)
var in_metrics_b_t: texture_2d<f32>;
@group(1) @binding(8)
var in_metrics_c_s: sampler;
@group(1) @binding(9)
var in_metrics_c_t: texture_2d<f32>;
@group(1) @binding(10)
var in_metrics_d_s: sampler;
@group(1) @binding(11)
var in_metrics_d_t: texture_2d<f32>;

fn simulate(in: VertexOutput) -> SimulationOutput {
    var out: SimulationOutput;
    if (1 == uniforms.is_active) {

        var color: vec4<f32>;

        if( uniforms.track_error == 1 ){
            let original_color = textureSample(in_color_t, in_color_s, in.tex_coords).rgb;
            var color_var = textureSample(in_metrics_c_t, in_metrics_c_s, in.tex_coords).rgb;
            var color_covar = textureSample(in_metrics_d_t, in_metrics_d_s, in.tex_coords).rgb;
            var dir_var = textureSample(in_metrics_a_t, in_metrics_a_s, in.tex_coords).ba;
            var dir_covar = textureSample(in_metrics_c_t, in_metrics_c_s, in.tex_coords).a;
            
            var S_col = covarMatFromVec3(color_var, color_covar);
            var S_pos = covarMatFromVec2(dir_var, dir_covar);

            // the 3.0 is used to strengthen the blur compared to the bloom effect
            color =  blur_with_error(in.tex_coords, in_color_s, in_color_t, uniforms.blur_factor * 3.0, uniforms.resolution, &S_col, &S_pos, in_metrics_c_s, in_metrics_c_t, in_metrics_d_s, in_metrics_d_t, in_metrics_a_s, in_metrics_a_t);

            color = vec4<f32>(applyBloom(color.rgb, uniforms.blur_factor/3.0, &S_col), color.a);
            color = lowerContrastBy(color, &S_col, uniforms.contrast_factor);
            out.color = color;

            //write back
            covarMatToVec3(S_col, &color_var, &color_covar);
            covarMatToVec2(S_pos, &dir_var, &dir_covar);

            // update the rgb values of the textures with the new data. do not touch the alpha
            out.color_change = vec4<f32>(textureSample(in_metrics_b_t, in_metrics_b_s, in.tex_coords).rgb + ( out.color.rgb - original_color), 0.0);
            out.color_uncertainty = vec4<f32>( color_var, 0.0 );
            out.deflection = vec4<f32>(textureSample(in_metrics_a_t, in_metrics_a_s, in.tex_coords).rg, dir_var);

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
            out.color_change =      textureSample(in_metrics_b_t, in_metrics_b_s, in.tex_coords);
            out.color_uncertainty = textureSample(in_metrics_c_t, in_metrics_c_s, in.tex_coords);
            out.deflection =        textureSample(in_metrics_a_t, in_metrics_a_s, in.tex_coords);
            out.covariances =       textureSample(in_metrics_d_t, in_metrics_d_s, in.tex_coords);
        }
    }
    out.depth = textureSample(in_depth_t, in_depth_s, in.tex_coords).r;
    return out;
}

@fragment
fn fs_color(in: VertexOutput) -> ColorOutput {
    let simulated = simulate(in);
    var out: ColorOutput;
    out.depth = simulated.depth;
    out.color = simulated.color;
    return out;
}

@fragment
fn fs_metrics_ab(in: VertexOutput) -> MetricsAbOutput {
    let simulated = simulate(in);
    let packed = unpackMetrics(
        simulated.deflection,
        simulated.color_change,
        simulated.color_uncertainty,
        simulated.covariances
    );
    var out: MetricsAbOutput;
    out.metrics_a = packMetricsA(packed);
    out.metrics_b = packMetricsB(packed);
    return out;
}

@fragment
fn fs_metrics_cd(in: VertexOutput) -> MetricsCdOutput {
    let simulated = simulate(in);
    let packed = unpackMetrics(
        simulated.deflection,
        simulated.color_change,
        simulated.color_uncertainty,
        simulated.covariances
    );
    var out: MetricsCdOutput;
    out.metrics_c = packMetricsC(packed);
    out.metrics_d = packMetricsD(packed);
    return out;
}
