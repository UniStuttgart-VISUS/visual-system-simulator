uniform sampler2D s_color;

in vec2 v_tex;
out vec4 rt_color;

void main() {
    rt_color = vec4(texture(s_color, v_tex).rgb, 1.0);
}
