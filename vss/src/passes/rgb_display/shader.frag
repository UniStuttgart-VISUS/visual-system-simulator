uniform sampler2D s_color;

in vec2 v_tex;
out vec4 rt_color;

void main() {
    vec3 color = texture(s_color, v_tex).rgb;
    rt_color = vec4(color, 1.0);
}
