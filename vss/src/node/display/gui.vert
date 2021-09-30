in vec2 a_pos;
in vec2 a_uv;
in vec4 a_col;

out vec2 v_tex;
out vec4 v_col;

uniform vec2 u_resolution_in;

void main() {
    v_tex = a_uv;
    v_col = a_col;
    vec2 v_pos = (a_pos/u_resolution_in)*2.0-1.0;
    gl_Position = vec4(v_pos.x, -v_pos.y, 0.0, 1.0);
}
