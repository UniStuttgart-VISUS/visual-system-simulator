uniform sampler2D s_color;
uniform sampler2D s_deflection;
uniform sampler2D s_color_change;
uniform sampler2D s_color_uncertainty;

uniform int u_vis_type;
uniform float u_heat_scale;

in vec2 v_tex;
out vec4 rt_color;

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

void main() {
    vec3 color = vec3(0.0);
    switch (u_vis_type) {
// TODO this has to be done in different color spaces, e.g with cielab and perhaps use deltaE_00
        case 0:
            color =  texture(s_color, v_tex).rgb;
            break;
        case 1: // directional uncertainty
            float bar =  pow(texture(s_color_change, v_tex).a, 2.0) + pow(texture(s_color_uncertainty, v_tex).a, 2.0);
            color = TurboColormap(sqrt(bar) * u_heat_scale);
            break;
        case 2: // color error
            vec3 err =  texture(s_color_change, v_tex).rgb;
            float combined_error = err.r * err.r + err.g * err.g + err.b * err.b;
            color = TurboColormap(sqrt(combined_error) * u_heat_scale);
            break;
        case 3: // color uncertainty
            vec3 foo =  texture(s_color_uncertainty, v_tex).rgb;
            float combined_variance = foo.r + foo.g +foo.b;
            color = TurboColormap(sqrt(combined_variance) * u_heat_scale);
            break;
    }
    
    if (v_tex.x < 0.0 || v_tex.y < 0.0 ||
        v_tex.x > 1.0 || v_tex.y > 1.0) {
        discard;
    }
    
    rt_color = vec4(color, 1.0);
}
