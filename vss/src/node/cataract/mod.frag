#include "common.glsl"

uniform int u_active;
uniform vec2 u_resolution;
uniform float u_blur_factor;
uniform float u_contrast_factor;
uniform int u_track_error;

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
        mat3 J = transpose(mat3(1.0,	0.0,	0.0, value, 1.0-value,	0.0, value, 0.0, 1.0-value));
        S = J*S*transpose(J);        
    } else if (color.g > color.r && color.g > color.b) {
        color.r += (color.g - color.r) * value;
        color.b += (color.g - color.b) * value;
        mat3 J = transpose(mat3(1-value, value,0, 0, 1,0, 0, value, 1-value));
        S = J*S*transpose(J);        
    } else{
        color.r += (color.b - color.r) * value;
        color.g += (color.b - color.g) * value;
        mat3 J = transpose(mat3(1-value, 0 , value, 0, 1-value, value, 0, 0, 1));
        S = J*S*transpose(J);
    }
}


void main() {
    if (1 == u_active) {

        vec4 color;

        if( u_track_error == 1 ){
            vec3 original_color = texture(s_color, v_tex).rgb;
            vec3 color_var = texture(s_color_uncertainty, v_tex).rgb;
            vec3 color_covar = texture(s_covariances, v_tex).rgb;
            vec2 dir_var = texture(s_deflection, v_tex).ba;
            float dir_covar = texture(s_covariances, v_tex).a;
            
            mat3 S_col = covarMatFromVec(color_var, color_covar);
            mat2 S_pos =covarMatFromVec(dir_var, dir_covar);

            // the 3.0 is used to strengthen the blur compared to the bloom effect
            color =  blur_with_error(v_tex, s_color, u_blur_factor * 3.0, u_resolution, S_col, S_pos, s_color_uncertainty, s_covariances, s_deflection);

            applyBloom(color.rgb, u_blur_factor/3, S_col);
            rt_color = color;
            lowerContrastBy(rt_color,S_col, u_contrast_factor);

            //write back
            covarMatToVec(S_col, color_var, color_covar);
            covarMatToVec(S_pos, dir_var, dir_covar);

            // update the rgb values of the textures with the new data. do not touch the alpha
            rt_color_change = vec4(texture(s_color_change, v_tex).rgb + ( rt_color.rgb - original_color), 0.0);
            rt_color_uncertainty = vec4( color_var, 0.0 );
            rt_deflection = vec4(texture(s_deflection, v_tex).rg, dir_var);

            rt_covariances = vec4(color_covar, dir_covar);
        }
        else{
            color =  blur(v_tex, s_color, u_blur_factor * 3.0, u_resolution);

            // since bloom and contrast are quite cheap, they do not have their own methods without error tracking
            mat3 unused_mat = mat3(0.0);
            applyBloom(color.rgb, u_blur_factor/3, unused_mat);
            rt_color = color;
            lowerContrastBy(rt_color,unused_mat, u_contrast_factor);
        }

    } else {
        rt_color =              vec4(texture(s_color, v_tex));

        if ( u_track_error == 1 ){
            rt_color_change =       texture(s_color_change, v_tex);
            rt_color_uncertainty =  texture(s_color_uncertainty, v_tex);
            rt_deflection =         texture(s_deflection, v_tex);
            rt_covariances = texture(s_covariances, v_tex);
        }
    }

    rt_depth = vec4(texture(s_depth, v_tex)).r;
}
