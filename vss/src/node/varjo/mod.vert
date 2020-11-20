uniform vec2 u_resolution_out;

out vec3 v_tex;

const vec2 vertices[6] = vec2[](
    vec2(0.0, 0.0),
    vec2(1.0, 0.0),
    vec2(1.0, 1.0),
    vec2(0.0, 0.0),
    vec2(1.0, 1.0),
    vec2(0.0, 1.0)
);

void main() {
    //have to use these coordinates for ndc so we can already pre-adjust them, z component serves to easier determine which matrix should be used
    v_tex = vec3(vertices[gl_VertexID % 6] * 2.0 - 1.0, gl_VertexID/6);
    vec2 pos = vertices[gl_VertexID % 6];

    float context_height = (u_resolution_out.x/2.0)/u_resolution_out.y; //assuming the context viewport has a ratio of 1:1

    if(gl_VertexID < 12){
        pos.y *= context_height;
    }else{
        pos.y *= 1.0-context_height;
        pos.y += context_height;
    }

    pos.x *= 0.5;
    if(gl_VertexID%12 >= 6){
        pos.x += 0.5;
    }

    gl_Position = vec4(pos * 2.0 - 1.0, 0.0, 1.0);
}
