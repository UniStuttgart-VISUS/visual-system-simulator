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
    vec2 ratio = u_resolution_out / u_resolution_in;
    vec2 scale = ratio / min(ratio.x, ratio.y);
    v_tex = scale * tex[gl_VertexID % 6] - 0.5 * scale + 0.5;    
    gl_Position = vec4(pos[gl_VertexID], 0.0, 1.0);
    
}
