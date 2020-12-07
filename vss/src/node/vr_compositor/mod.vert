uniform vec2 u_resolution_out;
uniform vec4 u_viewport;

out vec2 v_tex;

const vec2 vertices[6] = vec2[](
    vec2(0.0, 0.0),
    vec2(1.0, 0.0),
    vec2(1.0, 1.0),
    vec2(0.0, 0.0),
    vec2(1.0, 1.0),
    vec2(0.0, 1.0)
);

void main() {
    v_tex = vertices[gl_VertexID];
    vec2 pos = (vertices[gl_VertexID] * u_viewport.zw + u_viewport.xy)/u_resolution_out.xy;

    gl_Position = vec4(pos * 2.0 - 1.0, 0.0, 1.0);
}
