#define BLUR_SIZE 9

float normpdf(in float x) {
    return 0.39894 * exp(-0.5 * x * x / (49.0)) / 7.0;
}



mat3 covarMatFromVec(in vec3 var, in vec3 covar){
    return mat3(
        var.r       , covar.x     , covar.y      ,
        covar.x     , var.g       , covar.z      ,
        covar.y     , covar.z     , var.b
    ); 
}

mat2 covarMatFromVec(in vec2 var, in float covar){
    return mat2(
        var.x , covar , 
        covar , var.y       
    ); 
}

void covarMatToVec(in mat3 cvmat, inout vec3 var, inout vec3 covar){
    var     = vec3(cvmat[0][0],cvmat[1][1],cvmat[2][2]);
    covar   = vec3(cvmat[1][0],cvmat[2][0],cvmat[2][1]);
}

void covarMatToVec(in mat2 cvmat, inout vec2 var, inout float covar){
    var     = vec2(cvmat[0][0],cvmat[1][1]);
    covar   = cvmat[1][0];
}




//vec4 blur(in vec2 fragCoord, in sampler2D tex, float blur_scale, vec2 resolution, inout vec3 col_var, inout vec3 col_covar, in sampler2D col_vartex, inout vec2 dir_var,in sampler2D dir_vartex) {
vec4 blur(in vec2 fragCoord, in sampler2D tex, float blur_scale, vec2 resolution, inout mat3 S_col, inout mat2 S_pos, in sampler2D s_col_var, in sampler2D s_covariance, in sampler2D s_pos_var) {

    // declare stuff
    const int kSize = (BLUR_SIZE - 1) / 2;
    float kernel[BLUR_SIZE];
    vec3 final_colour = vec3(0.0);

    // create the 1-D kernel
    float Z = 0.0;
    for (int j = 0; j <= kSize; ++j) {
        kernel[kSize + j] = kernel[kSize - j] = normpdf(float(j));
    }

    // get the normalization factor (as the gaussian has been clamped)
    for (int j = 0; j < BLUR_SIZE; ++j) {
        Z += kernel[j];
    }

    //read out the texels
    for (int i = -kSize; i <= kSize; ++i) {
        for (int j = -kSize; j <= kSize; ++j) {
            final_colour += kernel[kSize + j] * kernel[kSize + i] * texture(tex, (fragCoord + vec2(float(i), float(j)) * blur_scale / resolution)).rgb;
        }
    }

    // the average of the weighted samples and hence the result
    vec3 avg = final_colour / (Z * Z);

    // s^2 = sum( (df/dx_i * s_i)^2 ) + s^2_blur
    //       ------------------------   ----------
    //              var_in               var_new
    vec3 col_var_in = vec3(0.0);
    vec3 col_covar_var_in = vec3(0.0);
    vec3 col_var_new = vec3(0.0);
    vec3 col_covar_new = vec3(0.0);
    vec2 dir_var_in = vec2(0.0);
    float dir_covar_in = 0.0;
    vec2 dir_var_new = vec2(0.0);
    float dir_covar_new = 0.0;

    vec3 pix = vec3(0.0);
    vec3 diff = vec3(0.0);
    float weight = 0;

    for (int i = -kSize; i <= kSize; ++i) {
        for (int j = -kSize; j <= kSize; ++j) {
            weight = kernel[kSize + j] * kernel[kSize + i] / (Z * Z);

            vec4 covars = texture(s_covariance, (fragCoord + vec2(float(i), float(j)) * blur_scale / resolution)) * weight;

            // propagated variance
            col_var_in += texture(s_col_var, (fragCoord + vec2(float(i), float(j)) * blur_scale / resolution)).rgb * weight;
            dir_var_in += texture(s_pos_var, (fragCoord + vec2(float(i), float(j)) * blur_scale / resolution)).ba * weight;

            col_covar_var_in += covars.rgb;
            dir_covar_in += covars.a;
            
            // new variance
            pix = texture(tex, (fragCoord + vec2(float(i), float(j)) * blur_scale / resolution)).rgb;
            diff = pix - avg;
            col_covar_new += vec3(diff.r*diff.g, diff.r*diff.b, diff.g*diff.b) *weight;

            diff = diff * diff;
            col_var_new += diff * weight;
        }
    }

    // although it is not used in the normpdf function, stretching the position vecor for sampling is comparable to stretching the gaussian
    // therefore, we can assume this factor multiplies the variance of the standard gaussian distribution ( sigma^2 = 1)
    dir_var_new = vec2(blur_scale*blur_scale/(resolution*resolution));

    // variances can now be added
    vec3 col_var = col_var_in + col_var_new;
    vec3 col_covar = col_covar_var_in + col_covar_new;

    vec2 dir_var = dir_var_in + dir_var_new;
    float dir_covar = dir_covar_in + dir_covar_new;


     S_col = covarMatFromVec(col_var, col_covar);
     S_pos = covarMatFromVec(dir_var, dir_covar);

    return vec4(avg,1.0);
}



float getPerceivedBrightness(in vec3 color) {
    vec3 weights = vec3(0.299, 0.587, 0.114);
    float brightness = dot(weights, color);
    return brightness;
}
/* covar:
    x: rg
    y: rb
    z: gb
*/
// Output: Although the brightness has variance in only one dimension, when multiplied by the color value, we have (co)variances in all three dimensions.
// Hence the bloom operation needs to calculate the variances of each channel separately
void applyBloom(inout vec3 color, in float blur_factor, inout mat3 S) {
// void applyBloom(inout vec3 color, in float blur_factor, inout vec3 var_in, inout vec3 cv) {
    // The constants adjust the contribution of each color to the perceived brightness, according to the amount of reception in the eye. See https://www.w3.org/TR/AERT/#color-contrast
    vec3 weights = vec3(0.299, 0.587, 0.114);
    float brightness = dot(weights, color);
   
    // the constant factor to scale the bloom compared to the blur
    float c =  blur_factor;

    // expands to color.r*weights.r+color.g*weights.g+color.b*weights.b
    float dotp =  dot(weights, color);

    /*
    Generates the jacobi matrix for color * (1. + brightness *  c) for all channels
    jacobian ([
        x_r*(1+(w_r*x_r+w_g*x_g+w_b*x_b)*c), 
        x_g*(1+(w_r*x_r+w_g*x_g+w_b*x_b)*c), 
        x_b*(1+(w_r*x_r+w_g*x_g+w_b*x_b)*c)
        ], [x_r,x_g,x_b]);
	)
    (Use wxMaxima to compute)
    */
    mat3 J = mat3(
        c*(dotp)+c*color.r*weights.r+1,	c*color.r*weights.g,	        c*color.r*weights.b,
	    c*color.g*weights.r,	        c*(dotp)+c*color.g*weights.g+1,	c*color.g*weights.b,
	    color.b*c*weights.r,	        color.b*c*weights.g,	        c*(dotp)+color.b*c*weights.b+1
    );

    mat3 Jt = transpose(J);

    // the resulting variance/covariance matrix
    S = J*S*Jt;

    float bloom_factor = 1. + brightness *  c;
    color = color * bloom_factor;
}