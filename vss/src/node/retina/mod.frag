#include "common.glsl"

// The resulting color will converge to this color in case of a glaucoma 
#define GLAUCOMA_COLOR vec4(0.02, 0.02, 0.02, 1.0);

uniform vec2 u_resolution;
uniform vec2 u_gaze;

uniform sampler2D s_color;
uniform sampler2D s_retina;
uniform sampler2D s_deflection;
uniform sampler2D s_color_change;
uniform sampler2D s_color_uncertainty;
uniform sampler2D s_covariances;


in vec2 v_tex;
out vec4 rt_color;
out vec4 rt_deflection;
out vec4 rt_color_change;
out vec4 rt_color_uncertainty;
out vec4 rt_covariances;


struct ErrorValues
{
  vec3 color_change;
  vec3 color_var;
  vec2 position_var;
};

void applyBlurAndBloom(inout vec4 color, in vec4 retina, inout ErrorValues ev) {
    float max_rgb = max(max(retina.r, retina.g), retina.b);
    float max_rgb_var = 0.0;
    if( retina.r >= retina.g && retina.r >= retina.b ){
        max_rgb_var = ev.color_var.r;
    }
    else if( retina.g >= retina.r && retina.g >= retina.b ){
        max_rgb_var = ev.color_var.g;
    }
    if( retina.b >= retina.g && retina.b >= retina.r ){
        max_rgb_var = ev.color_var.b;
    }
    // max_rgb has some uncertainty. Unfortunately it cannot be modeled in the if condition
    if (max_rgb < .75) {
        float blur_scale = (0.75 - (retina.r + retina.g + retina.b) / 3.0) * 5.0;
        //original calculation: 
        //      color = blur(v_tex, s_color, blur_scale, u_resolution);
        vec3 nonsense = vec3(0.0);
        color = blur(v_tex, s_color, blur_scale, u_resolution, ev.color_var, nonsense, s_color_uncertainty, ev.position_var, s_deflection);   
// TODO propagate directional uncertainty

        // apply bloom
        //original calculation: 
        //      color = color * (1.0 + getPerceivedBrightness(color.rgb) * (0.75 - max_rgb) * 0.5);
        float brightness = getPerceivedBrightness(color.rgb);
        float bloom_factor = 1. + brightness * (0.75 - max_rgb) * 0.5;
//         // f = 1+ x*(0.75-y) *0.5
//         // df/dx = -0.5(y-0.75)
//         // df/dy = -0.5x
//         // s^2 = sum( (df/dx_i * s_i)^2 )
//         float bloom_factor_variance = 
//              pow(-0.5 * (max_rgb-0.75) ,2.0) * brightness.y 
//            + pow(-0.5 * brightness.x,2.0)    * max_rgb_var
//             ;
// // TODO possibly covariance
//         ev.color_var = pow(bloom_factor ,2.0) * ev.color_var + (color.rgb*color.rgb) * bloom_factor_variance;
//         // do the bloom
        color = color * bloom_factor;
    }
}

void applyColorBlindness(inout vec4 color, in vec4 retina, inout ErrorValues ev) {
    mat3 neutral = mat3(1.0);
    mat3 weak_color = mat3(0.0);
    
    // we assume that perception of brightness is the same in cones+rods and rods only
    mat3 achromatopsia = mat3(
        0.299, 0.299, 0.299, // first column
        0.587, 0.587, 0.587, // second column
        0.114, 0.114, 0.114 // third column
    );

    // baseline, what is left of the weakest receptors (minimum percentage of all receptors)
    float neutral_factor = 0.0;
    
    // average of the other receptors in addition to baseline
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

    // factor missing in all receptors
    float achromatopsia_factor = 1.0 - neutral_factor - weak_color_factor;

    mat3 color_transformation = neutral * neutral_factor + weak_color * weak_color_factor + achromatopsia * achromatopsia_factor;
    color = vec4(color_transformation * color.rgb, color.a);

    // square all elements of the matrix, since for the variance, the weights of the weighted sum need to be squared
    vec3 zwei = vec3(2.0);
    mat3 squared_color_transformation =
        mat3(
            pow(color_transformation[0], zwei),
            pow(color_transformation[1], zwei),
            pow(color_transformation[2], zwei)
            );

    ev.color_var = squared_color_transformation * ev.color_var;
}

void applyNyctalopia(inout vec4 color, in vec4 retina, inout ErrorValues ev) {

    float brightness = getPerceivedBrightness(color.rgb);

    //float night_blindness_factor = getPerceivedBrightness(color.rgb) * 5.0;
    // float night_blindness_factor = brightness.x * 5;
    float night_blindness_factor = brightness * 5;
    // float nfb_var = brightness.y * 5 * 5; 
    if (night_blindness_factor < 1.0) {
        color = retina.a * color + (1.0 - retina.a) * night_blindness_factor * color;

        // f        = a * x + (1-a) * y * x
        // df/dx    = a - (1-a) * y
        // df/dy    = -(1-a) * x
        // s^2      = sum( (df/dx_i * s_i)^2 )
        // float dfdx_factor = retina.a - (1-retina.a) * night_blindness_factor;
        // vec3 dfdy_factor = vec3(-1 * (1-retina.a) ) * color.rgb;
        // ev.color_var = 
        //         pow(dfdx_factor, 2.0)       * ev.color_var 
        //       + pow(dfdy_factor, vec3(2.0)) * nfb_var
        //       ;
    }
}

void glaucoma(inout vec4 color, in vec4 retina_mask, inout ErrorValues ev) {
    // find the biggest value
    float maxValue = retina_mask.r > retina_mask.g ? retina_mask.r : retina_mask.g;
    maxValue = maxValue > retina_mask.b ? maxValue : retina_mask.b;
    maxValue = maxValue > retina_mask.a ? maxValue : retina_mask.a;
    // only takes effect when everything is lower than 0.25
    maxValue *= 4.0;
    if (maxValue < 1.0) {
        color = (maxValue) * (color) + (1.0 - maxValue) * GLAUCOMA_COLOR;
        ev.color_var *= maxValue*maxValue;
    }
}




void main() {
    vec2 gaze_offset = vec2(0.5, 0.5) - (u_gaze / u_resolution);
    vec4 retina_mask = texture(s_retina, v_tex + gaze_offset);

// TODO handle early termination direction
    /*
        Since there is no uncertainty att play here, we can overwrite all previous efforts with 0.
        The color changehas have to be added to the previously recorded changes.
        Since the resulting color is constant, it does not make sense to have uncertainties regarding position
    */
    if (retina_mask == vec4(0)) {
        vec4 color_diff = texture(s_color, v_tex) - GLAUCOMA_COLOR;
        rt_color_change =    
            vec4(
                texture(s_color_change, v_tex).rgb + color_diff.rgb,
                0.0
            );
        rt_color_uncertainty = 
            vec4(
                vec3(0.0),
                0.0
            );
        rt_color = GLAUCOMA_COLOR;
        return;
    }

    rt_color = texture(s_color, v_tex);
    vec3 original_color = rt_color.rgb;
    vec3 color_error = texture(s_color_change, v_tex).rgb;
    vec3 color_var = texture(s_color_uncertainty, v_tex).rgb;
    vec2 position_var = texture(s_deflection, v_tex).ba;
    ErrorValues ev = ErrorValues(
        color_error,
        color_var,
        position_var);

    applyBlurAndBloom(rt_color, retina_mask, ev);
    applyNyctalopia(rt_color, retina_mask, ev);
    applyColorBlindness(rt_color, retina_mask, ev);
    //glaucoma should be one of the last ones because it could decrease the brightness a lot
    glaucoma(rt_color, retina_mask, ev);

    vec3 color_diff = rt_color.rgb - original_color;
    rt_color_change = vec4(texture(s_color_change, v_tex).rgb + color_diff,0.0);
    rt_color_uncertainty = vec4(ev.color_var, 0.0);
    rt_deflection = vec4(texture(s_deflection, v_tex).rg, ev.position_var);
    // rt_deflection = texture(s_deflection, v_tex).rgba;
    rt_covariances = texture(s_covariances, v_tex);
}
