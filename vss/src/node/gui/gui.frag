//uniform sampler2D s_color;

uniform vec2 u_resolution_in;
uniform sampler2D u_tex;

in vec2 v_tex;
in vec4 v_col;
out vec4 rt_color;

void main() {
    rt_color = v_col * texture(u_tex, v_tex);
}
