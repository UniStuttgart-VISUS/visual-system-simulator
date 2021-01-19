#define BLUR_SIZE 9

float normpdf(in float x) {
    return 0.39894 * exp(-0.5 * x * x / (49.0)) / 7.0;
}


vec4 blur(in vec2 fragCoord, in sampler2D tex, float blur_scale, vec2 resolution, inout vec3 col_var, in sampler2D col_vartex, inout vec2 dir_var,in sampler2D dir_vartex) {
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
    //       ------------------------   ---------
    //              var_in               var_new
    vec3 col_var_in = vec3(0.0);
    vec3 col_var_new = vec3(0.0);
    vec2 dir_var_in = vec2(0.0);
    vec2 dir_var_new = vec2(0.0);
    vec3 pix = vec3(0.0);
    float weight = 0;
    for (int i = -kSize; i <= kSize; ++i) {
        for (int j = -kSize; j <= kSize; ++j) {
            weight = kernel[kSize + j] * kernel[kSize + i] / (Z * Z);
            // propagated variance
            col_var_in += texture(col_vartex, (fragCoord + vec2(float(i), float(j)) * blur_scale / resolution)).rgb * weight;// * weight;
            dir_var_in += texture(dir_vartex, (fragCoord + vec2(float(i), float(j)) * blur_scale / resolution)).ba * weight;// * weight;
            // new variance
            pix = texture(tex, (fragCoord + vec2(float(i), float(j)) * blur_scale / resolution)).rgb;
            pix = pix - avg;
            pix = pix * pix;
            col_var_new += pix * weight;
        }
    }

    // although it is not used in the normpdf function, stretching the position vecor for sampling is comparable to stretching the gaussian
    // therefore, we can assume this factor multiplies the variance of the standard gaussian distribution ( sigma^2 = 1)
    dir_var_new = vec2(blur_scale*blur_scale/(resolution*resolution));

    // variances can now be added
    col_var = col_var_in + col_var_new;
    dir_var = dir_var_in + dir_var_new;

    return vec4(avg,1.0);
}

vec2 getPerceivedBrightness(in vec3 color, in vec3 var_in) {
    // The constants adjust the contribution of each color to the perceived brightness, according to the amount of reception in the eye. See https://www.w3.org/TR/AERT/#color-contrast
    vec3 weights = vec3(0.299, 0.587, 0.114);
    float brightness = dot(weights, color);
    
    // since the brightness is the weighted sum of the vecor components, 
    // its uncertainty is propagated by summing the sqares of the weighted uncertainties
    // \sigma_f^2 = a^2\sigma_A^2 + b^2\sigma_B^2 + ...
    vec3 weighted_var = pow(weights, vec3(2.0)) * var_in;
    float variance = weighted_var.r + weighted_var.g + weighted_var.b;
    return vec2(brightness,variance);
}