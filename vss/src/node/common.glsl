#define BLUR_SIZE 9

float normpdf(in float x) {
    return 0.39894 * exp(-0.5 * x * x / (49.0)) / 7.0;
}

// blur function, blur-degree changeable by intensity parameter
vec4 blur(in vec2 fragCoord, in sampler2D tex, float blur_scale, vec2 resolution) {
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

    return vec4(final_colour / (Z * Z), 1.0);
}
