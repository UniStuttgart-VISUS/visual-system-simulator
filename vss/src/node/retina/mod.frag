#include "common.glsl"

#define GLAUCOMA_COLOR vec4(0.02, 0.02, 0.02, 1.0);

uniform vec2 u_resolution;
uniform vec2 u_gaze;

uniform sampler2D s_color;
uniform sampler2D s_retina;

in vec2 v_tex;
out vec4 rt_color;

float getLuminance(in vec4 color) {
    return dot(vec4(0.299, 0.587, 0.114, 0), color);
}

void applyBlurAndBloom(inout vec4 color, in vec4 retina) {
    float max_rgb = max(max(retina.r, retina.g), retina.b);
    if (max_rgb < .75) {
        float blur_scale = (0.75 - (retina.r + retina.g + retina.b) / 3.0) * 5.0;
        color = blur(v_tex, s_color, blur_scale, u_resolution);
        // apply bloom
        color = color * (1.0 + getLuminance(color) * (0.75 - max_rgb) * 0.5);
    }
}

void applyColorBlindness(inout vec4 color, in vec4 retina) {
    mat3 neutral = mat3(1.0);
    mat3 weak_color = mat3(0.0);
    mat3 achromatopsia = mat3(
        0.299, 0.299, 0.299, // first column
        0.587, 0.587, 0.587, // second column
        0.114, 0.114, 0.114 // third column
    );

    float neutral_factor = 0.0;
    float weak_color_factor = 0.0;

    if (retina.r < retina.g && retina.r < retina.b) {
        // protanopia
        weak_color = mat3(
            0.56667, 0.55833, 0.0,
            0.43333, 0.44167, 0.24167,
            0.0, 0.0, 0.75833);

        neutral_factor = retina.r;
        weak_color_factor = (retina.g + retina.b) / 2.0 - neutral_factor;
    } else if (retina.g < retina.r && retina.g < retina.b) {
        // deuteranopia
        weak_color = mat3(
            0.625, 0.70, 0.0,
            0.375, 0.30, 0.30,
            0.0, 0.0, 0.70);

        neutral_factor = retina.g;
        weak_color_factor = (retina.r + retina.b) / 2.0 - neutral_factor;
    } else {
        // tritanopia
        weak_color = mat3(
            0.95, 0.0, 0.0,
            0.5, 0.43333, 0.475,
            0.0, 0.56667, 0.525);

        neutral_factor = retina.b;
        weak_color_factor = (retina.r + retina.g) / 2.0 - neutral_factor;
    }

    float achromatopsia_factor = 1.0 - neutral_factor - weak_color_factor;

    mat4 color_transformation = mat4(
        neutral * neutral_factor + weak_color * weak_color_factor + achromatopsia * achromatopsia_factor);

    color = color_transformation * color;
}

void applyNyctalopia(inout vec4 color, in vec4 retina) {
    float night_blindness_factor = getLuminance(color) * 5.0;
    if (night_blindness_factor < 1.0) {
        color = retina.a * color + (1.0 - retina.a) * night_blindness_factor * color;
    }
}

void glaucoma(inout vec4 color, in vec4 retina_mask) {
    //find the biggest value
    float maxValue = retina_mask.r > retina_mask.g ? retina_mask.r : retina_mask.g;
    maxValue = maxValue > retina_mask.b ? maxValue : retina_mask.b;
    maxValue = maxValue > retina_mask.a ? maxValue : retina_mask.a;
    //only takes effect when everything is lower than 0.25
    maxValue *= 4.0;
    if (maxValue < 1.0) {
        color = (maxValue) * (color) + (1.0 - maxValue) * GLAUCOMA_COLOR;
    }
}

void main() {
    vec2 gaze_offset = vec2(0.5, 0.5) - (u_gaze / u_resolution);
    vec4 retina_mask = texture(s_retina, v_tex + gaze_offset);
    if (retina_mask == vec4(0)) {
        rt_color = GLAUCOMA_COLOR;
        return;
    }

    rt_color = texture(s_color, v_tex);

    applyBlurAndBloom(rt_color, retina_mask);
    applyNyctalopia(rt_color, retina_mask);
    applyColorBlindness(rt_color, retina_mask);
    //glaucoma should be one of the last ones because it could decrease the brightness a lot
    glaucoma(rt_color, retina_mask);
}
