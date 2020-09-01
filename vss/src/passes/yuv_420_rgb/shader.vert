uniform float u_rotation;
uniform vec2 u_resolution_in;
uniform vec2 u_resolution_out;

in vec2 a_pos;
in vec2 a_tex;
out vec2 v_tex;

void main() {
    v_tex = a_tex;
    v_tex.y = 1.0 - v_tex.y;
    v_tex = vec2(
        (v_tex.x - 0.5) * cos(u_rotation) - (v_tex.y - 0.5) * sin(u_rotation) + 0.5,
        (v_tex.x - 0.5) * sin(u_rotation) + (v_tex.y - 0.5) * cos(u_rotation) + 0.5);
    float in_ratio = u_resolution_in.x / u_resolution_in.y;
    float out_ratio = u_resolution_out.x / u_resolution_out.y;
    if (in_ratio != out_ratio) {
        if (in_ratio < out_ratio) {
            v_tex.y /= (out_ratio / in_ratio);
        } else {
            v_tex.x /= (in_ratio / out_ratio);
        }
    }
    gl_Position = vec4(a_pos, 0.0, 1.0);
}
