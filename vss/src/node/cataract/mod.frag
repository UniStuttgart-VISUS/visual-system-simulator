#include "common.glsl"

uniform int u_active;
uniform vec2 u_resolution;
uniform float u_blur_factor;
uniform float u_contrast_factor;
uniform sampler2D s_color;
uniform sampler2D s_depth;
uniform sampler2D s_deflection;
uniform sampler2D s_color_change;
uniform sampler2D s_color_uncertainty;


in vec2 v_tex;
out vec4 rt_color;
out float rt_depth;
//out vec4 rt_deflection;
out vec4 rt_color_change;
out vec4 rt_color_uncertainty;


void lowerContrastBy(inout vec4 color, inout vec3 color_var, float value) {
    if (color.r > color.g && color.r > color.b) {
        color.g += (color.r - color.g) * value;
        color.b += (color.r - color.b) * value;
        color_var.g = (color_var.r + color_var.g) * value * value;
        color_var.b = (color_var.r + color_var.b) * value * value;
    } else if (color.g > color.r && color.g > color.b) {
        color.r += (color.g - color.r) * value;
        color.b += (color.g - color.b) * value;
        color_var.r = (color_var.g + color_var.r) * value * value;
        color_var.b = (color_var.g + color_var.b) * value * value;
    } else {
        color.r += (color.b - color.r) * value;
        color.g += (color.b - color.g) * value;
        color_var.r = (color_var.b + color_var.r) * value * value;
        color_var.g = (color_var.b + color_var.g) * value * value;
    }
}

void main() {
    if (1 == u_active) {
        vec3 original_color = texture(s_color, v_tex).rgb;

        // inout parameter for bloom 
        vec3 color_var = vec3(0.0);
        // the 3.0 is used to strengthen the blur compared to the bloom effect
        vec4 color =  blur(v_tex, s_color, u_blur_factor * 3.0, u_resolution, color_var, s_color_uncertainty);        
        
        // this factor is used in the blur function to control the size of the blur
        // although it is not used in the normpdf function, stretching the position vecor for sampling is comparable to stretching the gaussian
        // therefore, we can assume this factor multiplies the sd of the standard gaussian distribution (1)
        //vec2 direction_uncertainty = vec2(u_blur_factor * 3.0);
        vec2 direction_uncertainty = vec2(0.0);


        // the /3. is used to weaken the bloom compared to the blur effect
        vec2 brightness = getPerceivedBrightness(color.rgb, color_var);
        
        float bloom_factor = 1. + brightness.x *  u_blur_factor / 3.;
        // f = a*x
        // s^2 = s^2_x * a^2
        float bloom_factor_variance = brightness.y * (u_blur_factor / 3. * u_blur_factor / 3.);

        /*
          This step produces large amounts of uncertainty in bright colors
        */
        
        rt_color = color * bloom_factor;
        // f = x_1 * x_+2
        // s^2 = s^2_1*x^2_2 + s^2_2*x^2_1) 
// TODO covariance  ?
        color_var =  color_var*(bloom_factor*bloom_factor)  + vec3(bloom_factor_variance)*(color.rgb*color.rgb);

        lowerContrastBy(rt_color,color_var, u_contrast_factor);

        vec3 color_diff = rt_color.rgb - original_color;        

        // update the rgb values of the textures with the new data. do not touch the alpha
        rt_color_change = 
            vec4(
                texture(s_color_change, v_tex).rgb + color_diff,
                texture(s_color_change, v_tex).a
            );
        rt_color_uncertainty = 
            vec4(
                color_var,
                texture(s_color_uncertainty, v_tex).a
            );

        // split the direction into the alpha components of the other two textures
        rt_color_change.a = direction_uncertainty.x;
        rt_color_change.a = direction_uncertainty.y;

        rt_depth = vec4(texture(s_depth, v_tex)).r;

    } else {
        rt_color =              vec4(texture(s_color, v_tex));
        rt_depth =              vec4(texture(s_depth, v_tex)).r;
        rt_color_change =       texture(s_color_change, v_tex);
        rt_color_uncertainty =  texture(s_color_uncertainty, v_tex);
    }
}
