#include "common.glsl"

uniform int u_active;
uniform vec2 u_resolution;
uniform float u_blur_factor;
uniform float u_contrast_factor;
uniform sampler2D s_color;
uniform sampler2D s_depth;

in vec2 v_tex;
out vec4 rt_color;
out float rt_depth;


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
        vec4 color =  blur(v_tex, s_color, u_blur_factor * 3.0, u_resolution);
        rt_color = color * computeBloomFactor(color, u_blur_factor / 3.);
        lowerContrastBy(rt_color, u_contrast_factor);
        rt_depth = vec4(texture(s_depth, v_tex)).r;
    } else {
        rt_color = vec4(texture(s_color, v_tex));
        rt_depth = vec4(texture(s_depth, v_tex)).r;
    }
}
