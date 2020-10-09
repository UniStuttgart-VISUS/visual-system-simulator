uniform int u_stereo;
uniform vec2 u_resolution_in;
uniform vec2 u_resolution_out;

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
    vec2 ratio = vec2(
        u_resolution_out.x / u_resolution_in.x,
        u_resolution_out.y / u_resolution_in.y);
    vec2 offset = 0.5 * max(vec2(ratio.x - ratio.y, ratio.y - ratio.x), 0.0);
    vec2 scale = ratio / min(ratio.x, ratio.y);
    v_tex = scale * tex[gl_VertexID % 6] - offset;
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
