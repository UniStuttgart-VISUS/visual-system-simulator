#define BLUR_SIZE 9

float normpdf(in float x) {
    return 0.39894 * exp(-0.5 * x * x / (49.0)) / 7.0;
}

vec4 blur(in vec2 fragCoord, in sampler2D tex, float blur_scale, vec2 resolution, inout vec3 col_var, inout vec3 col_covar, in sampler2D col_vartex, inout vec2 dir_var,in sampler2D dir_vartex) {
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
    vec2 dir_var_new = vec2(0.0);
    vec3 pix = vec3(0.0);
    vec3 diff = vec3(0.0);
    float weight = 0;

    for (int i = -kSize; i <= kSize; ++i) {
        for (int j = -kSize; j <= kSize; ++j) {
            weight = kernel[kSize + j] * kernel[kSize + i] / (Z * Z);
            // propagated variance
            col_var_in += texture(col_vartex, (fragCoord + vec2(float(i), float(j)) * blur_scale / resolution)).rgb * weight;
            dir_var_in += texture(dir_vartex, (fragCoord + vec2(float(i), float(j)) * blur_scale / resolution)).ba * weight;
            col_covar_var_in += texture(dir_vartex, (fragCoord + vec2(float(i), float(j)) * blur_scale / resolution)).rgb * weight;
            // new variance
            pix = texture(tex, (fragCoord + vec2(float(i), float(j)) * blur_scale / resolution)).rgb;
            diff = pix - avg;
            col_covar_new += vec3(diff.r*diff.g, diff.r*diff.b, diff.g*diff.b) *weight;
            //reuse for cariance
            diff = diff * diff;
            col_var_new += diff * weight;
        }
    }

    // although it is not used in the normpdf function, stretching the position vecor for sampling is comparable to stretching the gaussian
    // therefore, we can assume this factor multiplies the variance of the standard gaussian distribution ( sigma^2 = 1)
    dir_var_new = vec2(blur_scale*blur_scale/(resolution*resolution));

    // variances can now be added
    col_var = col_var_in + col_var_new;
    dir_var = dir_var_in + dir_var_new;
    col_covar = col_covar_var_in + col_covar_new;

    return vec4(avg,1.0);
}


mat3 covarMatFromVec(in vec3 var, in vec3 covar){
    return mat3(
        var.r       , covar.x     , covar.y      ,
        covar.x     , var.g       , covar.z      ,
        covar.y     , covar.z     , var.b
    ); 
}

void covarMatToVec(in mat3 cvmat, inout vec3 var, inout vec3 covar){
    var     = vec3(cvmat[0][0],cvmat[1][1],cvmat[2][2]);
    covar   = vec3(cvmat[1][0],cvmat[2][0],cvmat[2][1]);
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
   
    // to calculate the brightness of the bloom, we use the following formula
    // bloom = color.rgb * brightness
    // formula for brightness see above

    // with the genral formula for uncertainty of several (possibly dependent variables) 
    //u^2 = \sum_{i=1}^N\left(\frac{\partial f}{\partial x_i}\cdot u_i\right)^2 + 2\sum_{i=1}^{N-1}\sum_{k=i+1}^N\frac{\partial f}{\partial x_i}\ \frac{\partial f}{\partial x_k}\cdot u_{x_i, x_k}
    // we get
    // u_bloom^2 = u_g^2*(r*w_r+2*g*w_g+b*w_b)^2+g^2*u_r^2*w_r^2+2*(g*u_1*w_r*(r*w_r+2*g*w_g+b*w_b)+g*u_3*w_b*(r*w_r+2*g*w_g+b*w_b)+g^2*u_2*w_b*w_r)+g^2*u_b^2*w_b^2
    // Hint: you might want to plot these somewhere instead of trying to reason about it here

    // mat3 S = covarMatFromVec(var_in,cv);

    // Some shortcuts to bring the below formula on one screen page
    vec3 wsq = pow(weights, vec3(2.0));
    vec3 csq = pow(color, vec3(2.0));

    // The derivative of the "own" channel, e.g. for red: (2*r*w_r+g*w_g+b*w_b) 
    // dotp per dimension: r*w_r+g*w_g+b*w_b 
    // added vector: for each dimension its 2*_*_ part.
    vec3 ch_dvt =  vec3(dot(weights, color))+(weights * color);
    // float variance = vec3(
    //     pow(ch_dvt.r,2) * var_in.r + csq.r*wsq.g*var_in.g + csq.r*wsq.b*var_in.b + 2*( ch_dvt.r*color.r*weights.g*cv.x + ch_dvt.r*color.r*weights.b*cv.y + csq.r*weights.g*weights.b*cv.z)    ,
    //     pow(ch_dvt.g,2) * var_in.g + csq.g*wsq.r*var_in.r + csq.g*wsq.b*var_in.b + 2*( ch_dvt.g*color.g*weights.r*cv.x + ch_dvt.g*color.g*weights.b*cv.z + csq.g*weights.b*weights.r*cv.y)    ,
    //     pow(ch_dvt.b,2) * var_in.b + csq.b*wsq.r*var_in.r + csq.b*wsq.g*var_in.g + 2*( ch_dvt.b*color.b*weights.r*cv.y + ch_dvt.b*color.b*weights.g*cv.z + csq.b*weights.g*weights.r*cv.x)    
    //     );

    // mat3 J = mat3(
    //     ch_dvt.r            ,   weights.g*color.r   ,   weights.b*color.r   ,
    //     weights.r*color.g   ,   ch_dvt.g            ,   weights.b*color.g   ,
    //     weights.r*color.b   ,   weights.g*color.g   ,   ch_dvt.b
    // );

    // the constant factor to scale the bloom compared to the blur
    float c =  blur_factor / 3.;


    // expands to color.r*weights.r+color.g*weights.g+color.b*weights.b
    float dotp =  dot(weights, color);

    /*
    Generates the jacobi matrix for color * (1. + brightness *  c) for all channels
    jacobian ([
        r*(1+(w_r*r+w_g*g+w_b*b)*c), 
        g*(1+(w_r*r+w_g*g+w_b*b)*c), 
        b*(1+(w_r*r+w_g*g+w_b*b)*c)
        ], [r,g,b]);
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