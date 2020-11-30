uniform vec2 u_resolution_out;
uniform uint u_flow_index;

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
    vec2 pos = vertices[gl_VertexID];

    float context_height = (u_resolution_out.x/2.0)/u_resolution_out.y; //assuming the context viewport has a ratio of 1:1

    if(u_flow_index < 2){
        pos.y *= context_height;
    }else{
        pos.y *= 1.0-context_height;
        pos.y += context_height;
    }

    if((u_flow_index % 2) == 0){
        pos.x -= 1.0;
    }

    gl_Position = vec4(pos.x, pos.y * 2.0 - 1.0, 0.0, 1.0);
}
