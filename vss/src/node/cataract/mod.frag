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
uniform sampler2D s_covariances;


in vec2 v_tex;
out vec4 rt_color;
out float rt_depth;
out vec4 rt_deflection;
out vec4 rt_color_change;
out vec4 rt_color_uncertainty;
out vec4 rt_covariances;



void lowerContrastBy(inout vec4 color, inout mat3 S, float value) {
    if (color.r > color.g && color.r > color.b) {
        color.g += (color.r - color.g) * value;
        color.b += (color.r - color.b) * value;
        // jacobian ([ r, g+(r-g)*c, b+(r-b)*c ], [r,g,b]);
        mat3 J = mat3(1,	0,	0, value, 1-value,	0, value, 0, 1-value);
        S = J*S*transpose(J);
        
    } else if (color.g > color.r && color.g > color.b) {
        color.r += (color.g - color.r) * value;
        color.b += (color.g - color.b) * value;
        mat3 J = mat3(1-value, value,0, 0, 1,0, 0, value, 1-value);
        S = J*S*transpose(J);
    } else {
        color.r += (color.b - color.r) * value;
        color.g += (color.b - color.g) * value;
        mat3 J = mat3(1-value, 0 , value, 0, 1-value, value, 0, 0, 1);
        S = J*S*transpose(J);
    }
}

void main() {
    if (1 == u_active) {
        vec3 original_color = texture(s_color, v_tex).rgb;
        vec3 color_var = texture(s_color_uncertainty, v_tex).rgb;
        vec3 color_covar = texture(s_covariances, v_tex).rgb;
        vec2 dir_var = texture(s_deflection, v_tex).ba;

        // the 3.0 is used to strengthen the blur compared to the bloom effect
        // vec4 color =  blur(v_tex, s_color, u_blur_factor * 3.0, u_resolution, color_var, s_color_uncertainty, dir_var, s_deflection);        
        vec4 color =  blur(v_tex, s_color, u_blur_factor * 3.0, u_resolution, color_var, color_covar, s_color_uncertainty, dir_var, s_deflection);        
        
        mat3 S = covarMatFromVec(color_var, color_covar);

        applyBloom(color.rgb, u_blur_factor, S);
        rt_color = color;

        lowerContrastBy(rt_color,S, u_contrast_factor);

        covarMatToVec(S, color_var, color_covar);

        // update the rgb values of the textures with the new data. do not touch the alpha
        rt_color_change = vec4(texture(s_color_change, v_tex).rgb + ( rt_color.rgb - original_color), 0.0);
        rt_color_uncertainty = vec4( color_var, 0.0 );
        rt_deflection = vec4(texture(s_deflection, v_tex).rg, dir_var);

        rt_depth = vec4(texture(s_depth, v_tex)).r;
        rt_covariances = vec4(color_covar, texture(s_covariances, v_tex).a);


    } else {
        rt_color =              vec4(texture(s_color, v_tex));
        rt_depth =              vec4(texture(s_depth, v_tex)).r;
        rt_color_change =       texture(s_color_change, v_tex);
        rt_color_uncertainty =  texture(s_color_uncertainty, v_tex);
        rt_deflection =         texture(s_deflection, v_tex);
        rt_covariances = texture(s_covariances, v_tex);

    }
}
