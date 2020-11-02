#include "common.glsl"

uniform int u_active;
uniform vec2 u_resolution;
uniform float u_blur_factor;
uniform float u_contrast_factor;
uniform sampler2D s_color;
uniform sampler2D s_depth;
uniform sampler2D s_deflection;


in vec2 v_tex;
out vec4 rt_color;
out float rt_depth;
out vec4 rt_deflection;


float computeBloomFactor(in vec4 color, float blur) {
    float fact = 1. + getPerceivedBrightness(color.rgb) * blur;
    return fact;
}

// lowers the contrast of a color by the given percentage (value)
// This impl. moves the lower two color values closer to the value of the higher one
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
        // the 3.0 is used to strengthen the blur compared to the bloom effect
        vec4 color =  blur(v_tex, s_color, u_blur_factor * 3.0, u_resolution);
        // the /3. is used to weaken the bloom compared to the blur effect
        rt_color = color * computeBloomFactor(color, u_blur_factor / 3.);
        lowerContrastBy(rt_color, u_contrast_factor);
        rt_depth = vec4(texture(s_depth, v_tex)).r;
    } else {
        rt_color = vec4(texture(s_color, v_tex));
        rt_depth = vec4(texture(s_depth, v_tex)).r;
    }
    rt_deflection = texture(s_deflection, v_tex);
}
