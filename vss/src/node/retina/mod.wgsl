// The resulting color will converge to this color in case of a glaucoma 
const GLAUCOMA_COLOR: vec4<f32> = vec4<f32>(0.02, 0.02, 0.02, 1.0);

struct Uniforms{
    gaze_inv_proj: mat4x4<f32>,
    resolution: vec2<f32>,
    achromatopsia_blur_factor: f32,
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

@group(2) @binding(0)
var in_retina_t: texture_cube<f32>;
@group(2) @binding(1)
var in_retina_s: sampler;

struct ErrorValues{
  color_change: vec3<f32>, // TODO currently unused ?
  S_col: mat3x3<f32>,
  S_pos: mat2x2<f32>,
};

fn applyBlurAndBloom(color: vec4<f32>, retina: vec4<f32>, v_tex: vec2<f32>, ev: ptr<function, ErrorValues>, main_s: sampler, main_t: texture_2d<f32>, col_var_s: sampler, col_var_t: texture_2d<f32>, covariance_s: sampler, covariance_t: texture_2d<f32>, pos_var_s: sampler, pos_var_t: texture_2d<f32>) -> vec4<f32>{
    let max_rgb = max(max(retina.r, retina.g), retina.b);
    var max_rgb_var = 0.0;
    var newColor = color;
    var S_pos = (*ev).S_pos;
    var S_col = (*ev).S_col;

    if( retina.r >= retina.g && retina.r >= retina.b ){
        max_rgb_var = (*ev).S_col[0][0];
    }
    else if( retina.g >= retina.r && retina.g >= retina.b ){
        max_rgb_var = (*ev).S_col[1][1];
    }
    if( retina.b >= retina.g && retina.b >= retina.r ){
        max_rgb_var = (*ev).S_col[2][2];
    }
    // The decision of max_rgb has some uncertainty. 
    // Unfortunately it can hardly be modeled in the if condition, so we just use the decision as-is.
    // Adding Blur and Bloom when cones are missing is an approximation towards the loss of visual acuity and increased glare associated with achromatopsia
    if (max_rgb < .75) {
        // luckily we can assume that the retina values do not have any uncertainty attached
        let blur_scale = (0.75 - (retina.r + retina.g + retina.b) / 3.0) * 5.0 * uniforms.achromatopsia_blur_factor;
        if(uniforms.track_error == 1){
            newColor =  blur_with_error(v_tex, main_s, main_t, blur_scale, uniforms.resolution, &S_col, &S_pos, col_var_s, col_var_t, covariance_s, covariance_t, pos_var_s, pos_var_t);        
        }else{
            newColor =  blur(v_tex, main_s, main_t, blur_scale, uniforms.resolution);     
        }
        // apply bloom
        //original calculation: 
        //      color = color * (1.0 + getPerceivedBrightness(color.rgb) * (0.75 - max_rgb) * 0.5);

        // although this is not the blur factor per se, this is re-used in the bloom under this name
        let blur_factor = (0.75 - max_rgb) * 0.5;
        newColor = vec4<f32>(applyBloom(newColor.rgb, blur_factor, &S_col), newColor.a);
    }
    (*ev).S_pos = S_pos;
    (*ev).S_col = S_col;
    return newColor;
}

fn applyColorBlindness(color: vec4<f32>, retina: vec4<f32>, ev: ptr<function, ErrorValues>) -> vec4<f32> {
    var newColor = color;

    let neutral = mat3x3<f32>(
        vec3<f32>(1.0, 0.0, 0.0),
        vec3<f32>(0.0, 1.0, 0.0),
        vec3<f32>(0.0, 0.0, 1.0)
    );
    var weak_color = mat3x3<f32>(
        vec3<f32>(0.0, 0.0, 0.0),
        vec3<f32>(0.0, 0.0, 0.0),
        vec3<f32>(0.0, 0.0, 0.0)
    );
    
    // we assume that perception of brightness is the same in cones+rods and rods only
    let achromatopsia = mat3x3<f32>(
        vec3<f32>(0.299, 0.299, 0.299), // first column
        vec3<f32>(0.587, 0.587, 0.587), // second column
        vec3<f32>(0.114, 0.114, 0.114) // third column
    );

    // baseline, what is left of the weakest receptors (minimum percentage of all receptors)
    var neutral_factor = 0.0;
    
    // average of the other receptors in addition to baseline
    var weak_color_factor = 0.0;

    if (retina.r < retina.g && retina.r < retina.b) {
        // protanopia
        weak_color = mat3x3<f32>(
            vec3<f32>(0.56667, 0.55833, 0.0),
            vec3<f32>(0.43333, 0.44167, 0.24167),
            vec3<f32>(0.0, 0.0, 0.75833));

        neutral_factor = retina.r;
        weak_color_factor = (retina.g + retina.b) / 2.0 - neutral_factor;
    } else if (retina.g < retina.r && retina.g < retina.b) {
        // deuteranopia
        weak_color = mat3x3<f32>(
            vec3<f32>(0.625, 0.70, 0.0),
            vec3<f32>(0.375, 0.30, 0.30),
            vec3<f32>(0.0, 0.0, 0.70));

        neutral_factor = retina.g;
        weak_color_factor = (retina.r + retina.b) / 2.0 - neutral_factor;
    } else {
        // tritanopia
        weak_color = mat3x3<f32>(
            vec3<f32>(0.95, 0.0, 0.0),
            vec3<f32>(0.5, 0.43333, 0.475),
            vec3<f32>(0.0, 0.56667, 0.525));

        neutral_factor = retina.b;
        weak_color_factor = (retina.r + retina.g) / 2.0 - neutral_factor;
    }

    // factor missing in all receptors
    let achromatopsia_factor = 1.0 - neutral_factor - weak_color_factor;

    let testMat = mat3x3<f32>(
        vec3<f32>(1.0, 1.0, 1.0),
        vec3<f32>(1.0, 1.0, 1.0),
        vec3<f32>(1.0, 1.0, 1.0)
    );

    let color_transformation = (neutral * neutral_factor) + (weak_color * weak_color_factor) + (achromatopsia * achromatopsia_factor);
    newColor = vec4<f32>(color_transformation * newColor.rgb, newColor.a);

    if(uniforms.track_error == 1){
        // square all elements of the matrix, since for the variance, the weights of the weighted sum need to be squared
        let two = vec3<f32>(2.0, 2.0, 2.0);
        let squared_color_transformation =
            mat3x3<f32>(
                pow(color_transformation[0], two),
                pow(color_transformation[1], two),
                pow(color_transformation[2], two)
                );

        (*ev).S_col = squared_color_transformation * (*ev).S_col;
    }  
    return newColor;  
}

fn applyNyctalopia(color: vec4<f32>, retina: vec4<f32>, ev: ptr<function, ErrorValues>) -> vec4<f32> {
    var newColor = color;

    let brightness = getPerceivedBrightness(color.rgb);

    //float night_blindness_factor = getPerceivedBrightness(color.rgb) * 5.0;
    // float night_blindness_factor = brightness.x * 5;
    let night_blindness_factor = brightness * 5.0;
    // float nfb_var = brightness.y * 5 * 5; 
    if (night_blindness_factor < 1.0) {
        newColor = retina.a * color + (1.0 - retina.a) * night_blindness_factor * color;

        if(uniforms.track_error==1){
            // The weights from the brightness function
            let weights = vec3<f32>(0.299, 0.587, 0.114);

            /*
                To generate the Jacobi matrix, use color = retina.a * color + (1.0 - retina.a) * night_blindness_factor * color in the dimensions to derive
            */
            let J = transpose(mat3x3<f32>(
                vec3<f32>(5.0*(1.0-retina.a)*(newColor.b*weights.b+newColor.g*weights.g+newColor.r*weights.r)+5.0*(1.0-retina.a)*newColor.r*weights.r+retina.a,	5.0*(1.0-retina.a)*newColor.r*weights.g,	5.0*(1.0-retina.a)*newColor.r*weights.b),
                vec3<f32>(5.0*(1.0-retina.a)*newColor.g*weights.r,	5.0*(1.0-retina.a)*(newColor.b*weights.b+newColor.g*weights.g+newColor.r*weights.r)+5.0*(1.0-retina.a)*newColor.g*weights.g+retina.a,	5.0*(1.0-retina.a)*newColor.g*weights.b),
                vec3<f32>(5.0*(1.0-retina.a)*newColor.b*weights.r,	5.0*(1.0-retina.a)*newColor.b*weights.g,	5.0*(1.0-retina.a)*(newColor.b*weights.b+newColor.g*weights.g+newColor.r*weights.r)+5.0*(1.0-retina.a)*newColor.b*weights.b+retina.a)
            ));
            let Jt = transpose(J);
            (*ev).S_col = J*(*ev).S_col*Jt;
        }
    }
    return newColor;
}

fn glaucoma(color: vec4<f32>, retina: vec4<f32>, ev: ptr<function, ErrorValues>) -> vec4<f32> {
    var newColor = color;
    // find the biggest value
    var maxValue = max(max(retina.r, retina.g), max(retina.b, retina.a));
    // only takes effect when everything is lower than 0.25. 
    // This simulates a loss of resolution caused by less-sensitive photoreceptor types
    maxValue *= 4.0;
    if (maxValue < 1.0) {
        newColor = (maxValue) * (color) + (1.0 - maxValue) * GLAUCOMA_COLOR;
        (*ev).S_col *= maxValue*maxValue;
    }
    return newColor;
}

@fragment
fn fs_main(in: VertexOutput) -> FragmentOutput {
    var out: FragmentOutput;

    let ndc = vec4<f32>(vec2(in.tex_coords.x, 1.0 - in.tex_coords.y) * 2.0 - 1.0, 0.9, 1.0);
    let frag_dir = uniforms.gaze_inv_proj * ndc;

    let retina_mask = textureSample(in_retina_t, in_retina_s, normalize(frag_dir.xyz)/frag_dir.w);

    let original_color = textureSample(in_color_t, in_color_s, in.tex_coords);
    let deflection = textureSample(in_deflection_t, in_deflection_s, in.tex_coords);
    let color_change = textureSample(in_color_change_t, in_color_change_s, in.tex_coords);
    let color_uncertainty = textureSample(in_color_uncertainty_t, in_color_uncertainty_s, in.tex_coords);
    let covariances = textureSample(in_covariances_t, in_covariances_s, in.tex_coords);

    var color_var = color_uncertainty.rgb;
    var color_covar = covariances.rgb;
    var dir_var = deflection.ba;
    var dir_covar = covariances.a;

    out.color = original_color;

    /*
        Since there is no uncertainty at play here, we can overwrite all previous efforts with 0.
        The color changehas have to be added to the previously recorded changes.
        Since the resulting color is constant, it does not make sense to have uncertainties regarding position
    */

    if ((retina_mask.r == 0.0 && retina_mask.g == 0.0) && (retina_mask.b == 0.0 && retina_mask.a == 0.0)) {
        let color_diff = original_color - GLAUCOMA_COLOR;
        out.color_change = vec4<f32>(color_change.rgb + color_diff.rgb, 0.0 );
        out.color_uncertainty = vec4<f32>(0.0, 0.0, 0.0, 0.0);
        out.color = GLAUCOMA_COLOR;
    }else{
        let S_col = covarMatFromVec3(color_var, color_covar);
        let S_pos = covarMatFromVec2(dir_var, dir_covar);

        var ev = ErrorValues(
            color_change.rgb,
            S_col,
            S_pos
        );

        out.color = applyBlurAndBloom(out.color, retina_mask, in.tex_coords, &ev, in_color_s, in_color_t, in_color_uncertainty_s, in_color_uncertainty_t, in_covariances_s, in_covariances_t, in_deflection_s, in_deflection_t);
        out.color = applyNyctalopia(out.color, retina_mask, &ev);
        out.color = applyColorBlindness(out.color, retina_mask, &ev);
        //glaucoma should be one of the last ones because it could decrease the brightness a lot
        out.color = glaucoma(out.color, retina_mask, &ev);

        if(uniforms.track_error == 1){
            covarMatToVec3(ev.S_col, &color_var, &color_covar);
            covarMatToVec2(ev.S_pos, &dir_var, &dir_covar);
            
            let color_diff = out.color.rgb - original_color.rgb;
            out.color_change = vec4<f32>(color_change.rgb + color_diff, 0.0);
            out.color_uncertainty = vec4<f32>(color_var, 0.0);
            out.deflection = vec4<f32>(deflection.rg, dir_var);
            out.covariances = vec4<f32>(color_covar, dir_covar);
        }
    }

    return out;
}
