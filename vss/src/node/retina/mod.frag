#include "common.glsl"

// The resulting color will converge to this color in case of a glaucoma 
#define GLAUCOMA_COLOR vec4(0.02, 0.02, 0.02, 1.0);

uniform vec2 u_resolution;
uniform mat4 u_proj;
uniform float u_achromatopsia_blur_factor;

uniform sampler2D s_color;
uniform samplerCube s_retina;
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
  mat3 S_col;
  mat2 S_pos;
};

void applyBlurAndBloom(inout vec4 color, in vec4 retina, inout ErrorValues ev) {
    float max_rgb = max(max(retina.r, retina.g), retina.b);
    float max_rgb_var = 0.0;

    if( retina.r >= retina.g && retina.r >= retina.b ){
        max_rgb_var = ev.S_col[0][0];
    }
    else if( retina.g >= retina.r && retina.g >= retina.b ){
        max_rgb_var = ev.S_col[1][1];
    }
    if( retina.b >= retina.g && retina.b >= retina.r ){
        max_rgb_var = ev.S_col[2][2];
    }
    // The decision of max_rgb has some uncertainty. 
    // Unfortunately it can hardly be modeled in the if condition, so we just use the decision as-is.
    // Adding Blur and Bloom when cones are missing is an approximation towards the loss of visual acuity and increased glare associated with achromatopsia
    if (max_rgb < .75) {
        // luckily we can assume that the retina values do not have any uncertainty attached
        float blur_scale = (0.75 - (retina.r + retina.g + retina.b) / 3.0) * 5.0 * u_achromatopsia_blur_factor;
        color =  blur(v_tex, s_color, blur_scale, u_resolution, ev.S_col, ev.S_pos, s_color_uncertainty, s_covariances, s_deflection);        

        // apply bloom
        //original calculation: 
        //      color = color * (1.0 + getPerceivedBrightness(color.rgb) * (0.75 - max_rgb) * 0.5);

        // although this is not the blur factor per se, this is re-used in the bloom under this name
        float blur_factor = (0.75 - max_rgb) * 0.5;
        applyBloom(color.rgb, blur_factor, ev.S_col);
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

    ev.S_col = squared_color_transformation * ev.S_col;
}

void applyNyctalopia(inout vec4 color, in vec4 retina, inout ErrorValues ev) {

    float brightness = getPerceivedBrightness(color.rgb);

    //float night_blindness_factor = getPerceivedBrightness(color.rgb) * 5.0;
    // float night_blindness_factor = brightness.x * 5;
    float night_blindness_factor = brightness * 5;
    // float nfb_var = brightness.y * 5 * 5; 
    if (night_blindness_factor < 1.0) {
        color = retina.a * color + (1.0 - retina.a) * night_blindness_factor * color;

        // The weights from the brightness function
        vec3 weights = vec3(0.299, 0.587, 0.114);

        mat3 J = mat3(
            5*(1-retina.a)*(color.b*weights.b+color.g*weights.g+color.r*weights.r)+5*(1-retina.a)*color.r*weights.r+retina.a,	5*(1-retina.a)*color.r*weights.g,	5*(1-retina.a)*color.r*weights.b,
            5*(1-retina.a)*color.g*weights.r,	5*(1-retina.a)*(color.b*weights.b+color.g*weights.g+color.r*weights.r)+5*(1-retina.a)*color.g*weights.g+retina.a,	5*(1-retina.a)*color.g*weights.b,
            5*(1-retina.a)*color.b*weights.r,	5*(1-retina.a)*color.b*weights.g,	5*(1-retina.a)*(color.b*weights.b+color.g*weights.g+color.r*weights.r)+5*(1-retina.a)*color.b*weights.b+retina.a
	    );
        mat3 Jt = transpose(J);
        ev.S_col = J*ev.S_col*Jt;
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
        ev.S_col *= maxValue*maxValue;
    }
}


void main() {
    vec3 fragment_dir = normalize((u_proj * vec4(v_tex*2.0-1.0, 0.9, 1.0)).xyz);
    vec4 retina_mask = texture(s_retina, fragment_dir);
    //vec4 world_dir = inverse(u_proj) * vec4(v_tex * 2.0 - 1.0, 0.9, 1.0);
    //vec4 retina_mask = texture(s_retina, normalize(world_dir.xyz)/world_dir.w);

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
    vec3 color_covar = texture(s_covariances, v_tex).rgb;

    vec2 dir_var = texture(s_deflection, v_tex).ba;
    float dir_covar = texture(s_covariances, v_tex).a;

    mat3 S_col = covarMatFromVec(color_var, color_covar);
    mat2 S_pos =covarMatFromVec(dir_var, dir_covar);

    ErrorValues ev = ErrorValues(
        color_error,
        S_col,
        S_pos
    );

    applyBlurAndBloom(rt_color, retina_mask, ev);
    applyNyctalopia(rt_color, retina_mask, ev);
    applyColorBlindness(rt_color, retina_mask, ev);
    // //glaucoma should be one of the last ones because it could decrease the brightness a lot
    glaucoma(rt_color, retina_mask, ev);

    covarMatToVec(ev.S_col, color_var, color_covar);
    covarMatToVec(ev.S_pos, dir_var, dir_covar);
    
    vec3 color_diff = rt_color.rgb - original_color;
    rt_color_change = vec4(texture(s_color_change, v_tex).rgb + color_diff,0.0);
    rt_color_uncertainty = vec4(color_var, 0.0);
    rt_deflection = vec4(texture(s_deflection, v_tex).rg, dir_var);
    // rt_deflection = texture(s_deflection, v_tex).rgba;
    rt_covariances = vec4(color_covar, dir_covar);
}
