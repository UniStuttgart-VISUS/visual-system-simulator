uniform int u_stereo;

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
    if (u_stereo == 1) {
        if (gl_VertexID < 6) {
            gl_Position = vec4(pos[gl_VertexID] * vec2(0.5, 1.0) + vec2(0.5, 0.0), 0.0, 1.0);
        } else {
            gl_Position = vec4(pos[gl_VertexID % 6] * vec2(0.5, 1.0) - vec2(0.5, 0.0), 0.0, 1.0);
        }
    } else {
        gl_Position = vec4(pos[gl_VertexID], 0.0, 1.0);
    }
}
