let BLUR_SIZE: i32 = 9;

fn normpdf(x: f32) -> f32 {
    return 0.39894 * exp(-0.5 * x * x / (49.0)) / 7.0;
}

fn covarMatFromVec3(v: vec3<f32>, covar: vec3<f32>) -> mat3x3<f32>{
    return mat3x3<f32>(
        v.r         , covar.x     , covar.y      ,
        covar.x     , v.g         , covar.z      ,
        covar.y     , covar.z     , v.b
    );
}

fn covarMatFromVec2(v: vec2<f32>, covar: f32) -> mat2x2<f32>{
    return mat2x2<f32>(
        v.x, covar,
        covar, v.y
    );
}

fn covarMatToVec3(cvmat: mat3x3<f32>, v: ptr<function, vec3<f32>>, covar: ptr<function, vec3<f32>>){
    *v     = vec3<f32>(cvmat[0][0],cvmat[1][1],cvmat[2][2]);
    *covar = vec3<f32>(cvmat[1][0],cvmat[2][0],cvmat[2][1]);
}

fn covarMatToVec2(cvmat: mat2x2<f32>, v: ptr<function, vec2<f32>>, covar: ptr<function, f32>){
    *v     = vec2<f32>(cvmat[0][0],cvmat[1][1]);
    *covar = cvmat[1][0];
}

fn blur(fragCoord: vec2<f32>, main_s: sampler, main_t: texture_2d<f32>, blur_scale: f32, resolution: vec2<f32>) -> vec4<f32>{
    // declare stuff
    let kSize = (BLUR_SIZE - 1) / 2;
    var kernel: array<f32, BLUR_SIZE>;
    var final_colour = vec3<f32>(0.0);

    // create the 1-D kernel
    var Z = 0.0;
    for (var j: i32 = 0; j <= kSize; j++) {
        kernel[kSize - j] = normpdf(f32(j));
        kernel[kSize + j] = kernel[kSize - j];
    }

    // get the normalization factor (as the gaussian has been clamped)
    for (var j: i32 = 0; j < BLUR_SIZE; j++) {
        Z += kernel[j];
    }

    //read out the texels
    for (var i: i32 = -kSize; i <= kSize; i++) {
        for (var j: i32 = -kSize; j <= kSize; j++) {
            final_colour += kernel[kSize + j] * kernel[kSize + i] * textureSample(main_t, main_s, (fragCoord + vec2<f32>(f32(i), f32(j)) * blur_scale / resolution)).rgb;
        }
    }

    // the average of the weighted samples and hence the result
    let avg = final_colour / (Z * Z);
    return vec4<f32>(avg.r, avg.g, avg.b, 0.0);
}

//vec4 blur(in vec2 fragCoord, in sampler2D tex, float blur_scale, vec2 resolution, inout vec3 col_var, inout vec3 col_covar, in sampler2D col_vartex, inout vec2 dir_var,in sampler2D dir_vartex) {
fn blur_with_error(fragCoord: vec2<f32>, main_s: sampler, main_t: texture_2d<f32>, blur_scale: f32, resolution: vec2<f32>, S_col: ptr<function, mat3x3<f32>>, S_pos: ptr<function, mat2x2<f32>>, col_var_s: sampler, col_var_t: texture_2d<f32>, covariance_s: sampler, covariance_t: texture_2d<f32>, pos_var_s: sampler, pos_var_t: texture_2d<f32>) -> vec4<f32>{

    // declare stuff
    let kSize = (BLUR_SIZE - 1) / 2;
    var kernel: array<f32, BLUR_SIZE>;
    var final_colour = vec3<f32>(0.0);

    // create the 1-D kernel
    var Z = 0.0;
    for (var j: i32 = 0; j <= kSize; j++) {
        kernel[kSize - j] = normpdf(f32(j));
        kernel[kSize + j] = kernel[kSize - j];
    }

    // get the normalization factor (as the gaussian has been clamped)
    for (var j: i32 = 0; j < BLUR_SIZE; j++) {
        Z += kernel[j];
    }

    //read out the texels
    for (var i: i32 = -kSize; i <= kSize; i++) {
        for (var j: i32 = -kSize; j <= kSize; j++) {
            final_colour += kernel[kSize + j] * kernel[kSize + i] * textureSample(main_t, main_s, (fragCoord + vec2<f32>(f32(i), f32(j)) * blur_scale / resolution)).rgb;
        }
    }

    // the average of the weighted samples and hence the result
    let avg = final_colour / (Z * Z);
 
    var col_var_in = vec3<f32>(0.0);
    var col_covar_var_in = vec3<f32>(0.0);
    var col_var_new = vec3<f32>(0.0);
    var col_covar_new = vec3<f32>(0.0);
    var dir_var_in = vec2<f32>(0.0);
    var dir_covar_in = 0.0;
    var dir_var_new = vec2<f32>(0.0);
    var dir_covar_new = 0.0;

    var pix = vec3<f32>(0.0);
    var diff = vec3<f32>(0.0);
    var weight = 0.0;

    for (var i: i32 = -kSize; i <= kSize; i++) {
        for (var j: i32 = -kSize; j <= kSize; j++) {
            weight = kernel[kSize + j] * kernel[kSize + i] / (Z * Z);

            let covars = textureSample(covariance_t, covariance_s, (fragCoord + vec2<f32>(f32(i), f32(j)) * blur_scale / resolution)) * weight;

            // propagated variance
            col_var_in += textureSample(col_var_t, col_var_s, (fragCoord + vec2<f32>(f32(i), f32(j)) * blur_scale / resolution)).rgb * weight;
            dir_var_in += textureSample(pos_var_t, pos_var_s, (fragCoord + vec2<f32>(f32(i), f32(j)) * blur_scale / resolution)).ba * weight;

            col_covar_var_in += covars.rgb;
            dir_covar_in += covars.a;
            
            // new variance
            pix = textureSample(main_t, main_s, (fragCoord + vec2<f32>(f32(i), f32(j)) * blur_scale / resolution)).rgb;
            diff = pix - avg;
            col_covar_new += vec3<f32>(diff.r*diff.g, diff.r*diff.b, diff.g*diff.b) *weight;

            diff = diff * diff;
            col_var_new += diff * weight;
        }
    }

    // although it is not used in the normpdf function, stretching the position vecor for sampling is comparable to stretching the gaussian
    // therefore, we can assume this factor multiplies the variance of the standard gaussian distribution ( sigma^2 = 1)
    dir_var_new = vec2<f32>(blur_scale*blur_scale/(resolution*resolution));

    // variances can now be added
    let col_var = col_var_in + col_var_new;
    let col_covar = col_covar_var_in + col_covar_new;

    let dir_var = dir_var_in + dir_var_new;
    let dir_covar = dir_covar_in + dir_covar_new;

    *S_col = covarMatFromVec3(col_var, col_covar);
    *S_pos = covarMatFromVec2(dir_var, dir_covar);

    return vec4<f32>(avg.r, avg.g, avg.b, 1.0);
}

fn getPerceivedBrightness(color: vec3<f32>) -> f32{
    let weights = vec3<f32>(0.299, 0.587, 0.114);
    let brightness = dot(weights, color);
    return brightness;
}

/* covar:
    x: rg
    y: rb
    z: gb
*/
// Output: Although the brightness has variance in only one dimension, when multiplied by the color value, we have (co)variances in all three dimensions.
// Hence the bloom operation needs to calculate the variances of each channel separately
fn applyBloom(color: vec3<f32>, blur_factor: f32, S: ptr<function, mat3x3<f32>>) -> vec3<f32>{
    // The constants adjust the contribution of each color to the perceived brightness, according to the amount of reception in the eye. See https://www.w3.org/TR/AERT/#color-contrast
    let weights = vec3<f32>(0.299, 0.587, 0.114);
    let brightness = dot(weights, color);
   
    // the constant factor to scale the bloom compared to the blur
    let c = blur_factor;

    // expands to color.r*weights.r+color.g*weights.g+color.b*weights.b
    let dotp = dot(weights, color);

    let J = transpose(mat3x3<f32>(
        vec3<f32>(c*(dotp)+c*color.r*weights.r+1.0,	c*color.r*weights.g,	        c*color.r*weights.b),
	    vec3<f32>(c*color.g*weights.r,	        c*(dotp)+c*color.g*weights.g+1.0,	c*color.g*weights.b),
	    vec3<f32>(color.b*c*weights.r,	        color.b*c*weights.g,	        c*(dotp)+color.b*c*weights.b+1.0)
    ));

    let Jt = transpose(J);

    // the resulting variance/covariance matrix
    *S = J*(*S)*Jt;

    let bloom_factor = 1. + brightness *  c;
    return color * bloom_factor;
}