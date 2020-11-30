const float PI = 3.1415926535897932384626433832795;

//uniform mat4 u_view_matrices[4]; //TODO use array of matrices if possible
//uniform mat4 u_proj_matrices[4]; //TODO
uniform mat4 u_view_context_l;
uniform mat4 u_view_context_r;
uniform mat4 u_view_focus;
uniform mat4 u_proj_context;
uniform mat4 u_proj_focus;
uniform uint u_right_eye;

uniform sampler2D s_color;

in vec3 v_tex;
out vec4 rt_color;

void main() {
    mat4 view, projection;
    if(v_tex.z == 0.0){
        if(u_right_eye == 1){
            view = u_view_context_r;
        }else{
            view = u_view_context_l;
        }
        projection = u_proj_context;
    }else{
        view = u_view_focus;
        projection = u_proj_focus;
    }

    //get head position to fix it
    vec4 head = (u_view_context_l * vec4(0.0, 0.0, 0.0, 1.0) + u_view_context_r * vec4(0.0, 0.0, 0.0, 1.0)) / 2.0;
    mat4 translate_head = mat4(1.0, 0.0, 0.0, 0.0,  0.0, 1.0, 0.0, 0.0,  0.0, 0.0, 1.0, 0.0, -head.x, -head.y, -head.z, 1.0);

    vec4 ndc = vec4(v_tex.xy, 0.9, 1.0);
    vec4 world_dir = inverse(projection * (translate_head*view)) * ndc;
    world_dir.xyz = normalize(world_dir.xyz)/world_dir.w;
    vec2 tex = vec2(atan(world_dir.z, world_dir.x) + PI, acos(-world_dir.y)) / vec2(2.0 * PI, PI);

    rt_color = vec4(texture(s_color, tex).rgb, 1.0);
}
