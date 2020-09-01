#define BLUR_SIZE 9

uniform int u_active;
uniform vec2 u_resolution;
uniform float u_blur_factor;
uniform float u_contrast_factor;
uniform sampler2D s_color;

in vec2 v_tex;
out vec4 rt_color;

float normpdf(in float x) {
    return 0.39894 * exp(-0.5 * x * x / (49.0)) / 7.0;
}

// blur function, blur-degree changeable by intensity parameter
vec3 blur(in vec2 fragCoord, float blur_scale) {
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
            final_colour += kernel[kSize + j] * kernel[kSize + i] * texture(s_color, (fragCoord + vec2(float(i), float(j)) * blur_scale / u_resolution)).rgb;
        }
    }

    return final_colour / (Z * Z);
}

float computeBloomFactor(in vec4 color, float blur) {
    float fact = 1. + (color.r * 0.299 + color.g * 0.587 + color.b * 0.114) * blur;
    return fact;
}

// lowers the contrast of a color by the given percentage (value)
void lowerContrastBy(inout vec4 color, float value) {
    if (color.r > color.g && color.r > color.b) {
        color.g += (color.r - color.g) * value;
        color.b += (color.r - color.b) * value;
    } else if (color.g > color.r && color.g > color.b) {
        color.r += (color.g - color.r) * value;
        color.b += (color.g - color.b) * value;
    } else {
        color.r += (color.b - color.r) * value;
        color.g += (color.b - color.g) * value;
    }
}

void main() {
    if (1 == u_active) {
        vec2 xy = v_tex / u_resolution;
        vec3 result = blur(v_tex, u_blur_factor * 3.0);
        vec4 pixel = vec4(result, 1.0);
        rt_color = pixel * computeBloomFactor(pixel, u_blur_factor / 3.);
        lowerContrastBy(rt_color, u_contrast_factor);
    } else {
        rt_color = vec4(texture(s_color, v_tex));
    }
}
