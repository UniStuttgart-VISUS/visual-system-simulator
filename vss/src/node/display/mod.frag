#include "common.glsl"

uniform sampler2D s_color;
uniform sampler2D s_deflection;
uniform sampler2D s_color_change;
uniform sampler2D s_color_uncertainty;
uniform sampler2D s_original;
uniform sampler2D s_covariances;

uniform vec2 u_resolution_in;


uniform int u_vis_type;
uniform int u_flow_idx;
uniform float u_heat_scale;
uniform int u_track_error;

in vec2 v_tex;
out vec4 rt_color;


// Viridis colormap, based on https://github.com/glslify/glsl-colormap
const float e0 = 0.0;
const vec4 v0 = vec4(0.26666666666666666,0.00392156862745098,0.32941176470588235,1);
const float e1 = 0.13;
const vec4 v1 = vec4(0.2784313725490196,0.17254901960784313,0.47843137254901963,1);
const float e2 = 0.25;
const vec4 v2 = vec4(0.23137254901960785,0.3176470588235294,0.5450980392156862,1);
const float e3 = 0.38;
const vec4 v3 = vec4(0.17254901960784313,0.44313725490196076,0.5568627450980392,1);
const float e4 = 0.5;
const vec4 v4 = vec4(0.12941176470588237,0.5647058823529412,0.5529411764705883,1);
const float e5 = 0.63;
const vec4 v5 = vec4(0.15294117647058825,0.6784313725490196,0.5058823529411764,1);
const float e6 = 0.75;
const vec4 v6 = vec4(0.3607843137254902,0.7843137254901961,0.38823529411764707,1);
const float e7 = 0.88;
const vec4 v7 = vec4(0.6666666666666666,0.8627450980392157,0.19607843137254902,1);
const float e8 = 1.0;
const vec4 v8 = vec4(0.9921568627450981,0.9058823529411765,0.1450980392156863,1);
vec3 ViridisColormap(in float x_0) {
    float a0 = smoothstep(e0,e1,x_0);
  float a1 = smoothstep(e1,e2,x_0);
  float a2 = smoothstep(e2,e3,x_0);
  float a3 = smoothstep(e3,e4,x_0);
  float a4 = smoothstep(e4,e5,x_0);
  float a5 = smoothstep(e5,e6,x_0);
  float a6 = smoothstep(e6,e7,x_0);
  float a7 = smoothstep(e7,e8,x_0);
  return max(mix(v0,v1,a0)*step(e0,x_0)*step(x_0,e1),
    max(mix(v1,v2,a1)*step(e1,x_0)*step(x_0,e2),
    max(mix(v2,v3,a2)*step(e2,x_0)*step(x_0,e3),
    max(mix(v3,v4,a3)*step(e3,x_0)*step(x_0,e4),
    max(mix(v4,v5,a4)*step(e4,x_0)*step(x_0,e5),
    max(mix(v5,v6,a5)*step(e5,x_0)*step(x_0,e6),
    max(mix(v6,v7,a6)*step(e6,x_0)*step(x_0,e7),mix(v7,v8,a7)*step(e7,x_0)*step(x_0,e8)
  ))))))).rgb;
}

// from https://gist.github.com/mikhailov-work/0d177465a8151eb6ede1768d51d476c7
vec3 TurboColormap(in float x) {
  const vec4 kRedVec4 = vec4(0.13572138, 4.61539260, -42.66032258, 132.13108234);
  const vec4 kGreenVec4 = vec4(0.09140261, 2.19418839, 4.84296658, -14.18503333);
  const vec4 kBlueVec4 = vec4(0.10667330, 12.64194608, -60.58204836, 110.36276771);
  const vec2 kRedVec2 = vec2(-152.94239396, 59.28637943);
  const vec2 kGreenVec2 = vec2(4.27729857, 2.82956604);
  const vec2 kBlueVec2 = vec2(-89.90310912, 27.34824973);
  
  x = clamp(x, 0.0, 1.0);

  vec4 v4 = vec4( 1.0, x, x * x, x * x * x);
  vec2 v2 = v4.zw * v4.z;
  return vec3(
    dot(v4, kRedVec4)   + dot(v2, kRedVec2),
    dot(v4, kGreenVec4) + dot(v2, kGreenVec2),
    dot(v4, kBlueVec4)  + dot(v2, kBlueVec2)
  );
}



// Watson et al approximate the distance on the retina to degrees conversion as a linear equation
// Since we do not have the distance in mm from the center but in [0; 0.5], 
// and the lens maps the image space to +/- 90°, we can map 90 deg to 1.0 linear
float dist_to_deg(in float dist){
    return 2*dist*90;
}
float deg_to_dist(in float deg){
    return deg/(90*2);
}

// depending on which principal meridians enclose the point, we need to calculate with different values for RGCf density
// finding the meridians is a bit tricky for the individual eyes, so i did some "art"
/*
                                 ┌─────┐
               2         eye:l   │     │   eye:r          2
            superior             │     │               superior
               │                 │     │                  │
               │                 │     │                  │
    temporal   │                 │     │                  │     temporal
1  ────────────┼───────────── 3  │     │   3  ────────────┼───────────── 1
               │      nasal      │     │       nasal      │
               │                 │     │                  │
               │                 │     │                  │
           inferior              │     │              inferior
                                 └─────┘
               4                  nose                    4
*/
int[2] meridians_from_pos(in vec2 pos){

    // flip the x axis if we are in the left eye
    if( u_flow_idx > 0 ){
        pos.x *= -1.0;
    }

    // temporal, superior: 1, 2
    if(pos.x >= 0 && pos.y >= 0){ 
        return int[2](0,1);
    }
    // nasal, superior: 2, 3
    else if(pos.x <= 0 && pos.y >= 0){
        return int[2](2,1);
    }
    // nasal, inferior: 3, 4
    else if(pos.x <= 0 && pos.y <= 0){
        return int[2](2,3);
    }
    // temporal, inferior: 1, 4
    else{ // equal to if(pos.x >= 0 && pos.y <= 0){
        return int[2](0,3);
    }
}  

// dont get tricked by columds vs rows
const mat4 watsons_params = mat4(
    //a     //r_2   //r_e   // filler
    0.9851, 1.058,  22.14,  0.0,
    0.9935, 1.035,  16.35,  0.0,
    0.9729, 1.084,  7.633,  0.0,
    0.996,  0.9932, 12.13,  0.0
);

const float max_d_rgcf = 33000.0;

float weighted_density_meridian(in float x, in float r_xy, in vec3 params){
    float rel_density =  params.x * pow(1.0+(r_xy/params.y),-2.0) + (1.0-params.x) * exp(-1.0 * r_xy/params.z);
    float weighted_density = pow(x, 2.0) * pow(rel_density, 2.0);
    return weighted_density;
}

// calculate the spacing between RGCf
float spacing(in vec2 pos) {

    float r_xy = dist_to_deg(sqrt(pos.x * pos.x + pos.y * pos.y));
    
    // lookup the parameters for the enclosing meridians
    int[2] meridians = meridians_from_pos(pos);
    vec3 m_x_params = watsons_params[meridians[0]].xyz;
    vec3 m_y_params = watsons_params[meridians[1]].xyz;
    float x = dist_to_deg(pos.x);
    float y = dist_to_deg(pos.y);

    // get the weighted densities at the meridians
    float wdx = weighted_density_meridian(abs(x), r_xy, m_x_params);
    float wdy = weighted_density_meridian(abs(y), r_xy, m_y_params);

    // calculate the density from the two components
    float density = max_d_rgcf * (1.0/r_xy) * sqrt( wdx + wdy );

    float spacing = sqrt(1.0/(density));
    spacing = deg_to_dist(spacing);

    // scale the whole calculation to match with our capabilities:
    // we do not want to simulate ganglion cells cmaller than a pixel 
    float desired_res = sqrt(max_d_rgcf)*90;
    float available_res = min(u_resolution_in.x, u_resolution_in.y);
    float scale_factor = desired_res/available_res;

    spacing *= scale_factor;
    return spacing;
}

vec3 RetinalGanglion(){

    vec2 loc = v_tex;

    // want to see the density? use this!
    //return vec3(TurboColormap(16*spacing(loc-0.5)));

    float len = 0.5;
    vec2 center = vec2(0.5);
    
    // subdivide until we have the required gradnularity
    for (int i=0; i<16; i++) {  
        if( 2*len < spacing(center-0.5) ){
            break;
        } 
        len/=2.0;
        center = center + step(center,loc)*len - step(loc,center)*len;
    }
    
    // make sure we sample at least one pixel around the own one
    float sample_distance = max(len, 1.0/u_resolution_in.y);

    float gray_own = getPerceivedBrightness(texture(s_color, center).rgb);
    float gray_others = 0;

    // sample 8 pixels with the distance calculated above
    for (int i = -1; i <= 1; i++) {
        for (int j = -1; j <= 1; j++) {
            if(i!=0||j!=0){ // exclude i=j=0
                gray_others += getPerceivedBrightness(texture(s_color, center + vec2(float(i), float(j)) * sample_distance ).rgb);
            }
        }
    }
    gray_others /= 8;

    float lum_diff = gray_own - gray_others;

    float threshold = 0.01 * u_heat_scale;
    if( lum_diff > threshold){        // ON center
        return vec3(1.0,0.0,0.0) * 8 * ( lum_diff );
    }
    else if( lum_diff < (-1.0 * threshold) ){        // OFF center
        return vec3(0.0,0.0,1.0) * 8 * abs( lum_diff );
    }
    else{
        return vec3(0.0, 0.0, 0.0);
    }

}

void main() {
    vec3 color = vec3(0.0);
    switch (u_vis_type) {
// TODO this has to be done in different color spaces, e.g with cielab and perhaps use deltaE_00
        case 0:  // simulated image
            color =  texture(s_color, v_tex).rgb;
            break;
        case 1: // directional uncertainty
            //float bar =  pow(texture(s_deflection, v_tex).r, 2.0) + pow(texture(s_deflection, v_tex).g, 2.0);
            float bar =  pow(texture(s_deflection, v_tex).b, 2.0) + pow(texture(s_deflection, v_tex).a, 2.0);
            color = ViridisColormap(sqrt(bar) * 1080 * u_heat_scale);
            // color = vec3(sqrt(bar) * 1080 * u_heat_scale);
            break;
        case 2: // color error
            vec3 err =  texture(s_color_change, v_tex).rgb;
            float combined_error = err.r * err.r + err.g * err.g + err.b * err.b;
            color = ViridisColormap(sqrt(combined_error) * u_heat_scale);
            break;
        case 3: // color uncertainty
            // vec3 cvm_var = texture(s_color_uncertainty, v_tex).rgb;
            // vec3 cvm_covar = texture(s_covariances, v_tex).rgb;

            // mat3 cvm = mat3(
            //     cvm_var.r, cvm_covar.r, cvm_covar.g, 
            //     cvm_covar.r, cvm_var.g, cvm_covar.b,
            //     cvm_covar.g, cvm_covar.b, cvm_var.b
            // );
            // color = ViridisColormap(abs(determinant(cvm)) * u_heat_scale);

            vec3 foo =  texture(s_color_uncertainty, v_tex).rgb;
            float combined_variance = foo.r + foo.g +foo.b;
            color = ViridisColormap(sqrt(combined_variance) / u_heat_scale);
            
            break;
        case 4: // original image
            color =  texture(s_original, v_tex).rgb;
            break;
        case 5:
        case 6:
            // without positional correction
            vec3 unc_val =  texture(s_color_uncertainty, v_tex).rgb;
            // with positional correction
            // vec3 unc_val =  texture(s_color_uncertainty, v_tex-texture(s_deflection, v_tex).rg).rgb;
            float hm_val = unc_val.r + unc_val.g + unc_val.b;
            hm_val = sqrt(hm_val) * u_heat_scale;
            if (hm_val > 0.25) {
                color = ViridisColormap(hm_val);
            }
            else{
                if(u_vis_type == 5){
                    color = texture(s_color, v_tex).rgb;
                }
                else{
                    color = texture(s_original, v_tex).rgb;
                }
            }
            break;
        case 7: // retinal ganglion ON/OFF
            color = RetinalGanglion();  

            // float cov =  texture(s_color_uncertainty, v_tex).r + texture(s_color_uncertainty, v_tex).g + texture(s_color_uncertainty, v_tex).b 
            // -(texture(s_covariances, v_tex).r + texture(s_covariances, v_tex).g + texture(s_covariances, v_tex).b);

            // // // float cov =  texture(s_covariances, v_tex).r + texture(s_covariances, v_tex).g + texture(s_covariances, v_tex).b;

            // color = ViridisColormap(sqrt(abs(cov))* u_heat_scale);

            break;

    }
    
    if (v_tex.x < 0.0 || v_tex.y < 0.0 ||
        v_tex.x > 1.0 || v_tex.y > 1.0) {
        discard;
    }
    
    rt_color = vec4(color, 1.0);
}
