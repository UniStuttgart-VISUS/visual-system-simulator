uniform int u_vflip;

uniform sampler2D s_rgb;

in vec2 v_tex;
out vec4 rt_color;

void main() {
    if (u_vflip == 1) {
        vec2 v_tex_flipped = vec2(v_tex.x, 1.0 - v_tex.y);
        rt_color = vec4(texture(s_rgb, v_tex_flipped).rgb, 1.0);
    } else {
        rt_color = vec4(texture(s_rgb, v_tex).rgb, 1.0);
    }
}
