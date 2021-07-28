in vec2 a_pos;
in vec3 a_col;

out vec2 v_tex;

void main() {
    v_tex = vec2(0.0, 0.0);
    gl_Position = vec4(a_pos, 0.0, 1.0);
}
