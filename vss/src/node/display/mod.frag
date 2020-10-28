uniform sampler2D s_color;
uniform sampler2D s_deflection;

uniform int u_vis_type;
uniform float u_heat_scale;

in vec2 v_tex;
out vec4 rt_color;

void main() {
    vec3 color = vec3(0.0);
    switch (u_vis_type) {
        case 0:
            color =  texture(s_color, v_tex).rgb;
            break;
        case 1:
            color =  texture(s_deflection, v_tex).rgb * u_heat_scale;
            break;
    }
    
    if (v_tex.x < 0.0 || v_tex.y < 0.0 ||
        v_tex.x > 1.0 || v_tex.y > 1.0) {
        discard;
    }
    
    rt_color = vec4(color, 1.0);
}
