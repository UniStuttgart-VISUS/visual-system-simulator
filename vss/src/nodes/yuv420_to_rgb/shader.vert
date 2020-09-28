uniform int u_stereo;
uniform float u_rotation;
uniform vec2 u_aspect_in;
uniform vec2 u_aspect_out;

out vec2 v_tex;

const vec2 pos[6] = vec2[](
    vec2(-1.0, -1.0),
    vec2(1.0, -1.0),
    vec2(1.0, 1.0),
    vec2(-1.0, -1.0),
    vec2(1.0, 1.0),
    vec2(-1.0, 1.0)
);
const vec2 tex[6] = vec2[](
    vec2(0.0, 0.0),
    vec2(1.0, 0.0),
    vec2(1.0, 1.0),
    vec2(0.0, 0.0),
    vec2(1.0, 1.0),
    vec2(0.0, 1.0)
);

void main() {
    v_tex = tex[gl_VertexID % 6];
    v_tex.y = 1.0 - v_tex.y;
    v_tex = vec2(
        (v_tex.x - 0.5) * cos(u_rotation) - (v_tex.y - 0.5) * sin(u_rotation) + 0.5,
        (v_tex.x - 0.5) * sin(u_rotation) + (v_tex.y - 0.5) * cos(u_rotation) + 0.5);
    if (u_aspect_in < u_aspect_out) {
        v_tex.y /= (u_aspect_out / u_aspect_in);
    } else if  (u_aspect_in > u_aspect_out) {
        v_tex.x /= (u_aspect_in / u_aspect_out);
    }
    if (u_stereo == 1) {
        if (gl_VertexID < 6) {
            gl_Position = vec4(pos[gl_VertexID] * vec2(0.5, 1.0) + vec2(0.5, 0.0), 0.0, 1.0);
        } else {
            gl_Position = vec4(pos[gl_VertexID] * vec2(0.5, 1.0) - vec2(0.5, 0.0), 0.0, 1.0);
        }
    } else {
        gl_Position = vec4(pos[gl_VertexID], 0.0, 1.0);
    }
}
