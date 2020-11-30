const float PI = 3.1415926535897932384626433832795;

uniform mat4 u_view_context_l;
uniform mat4 u_view_context_r;
uniform mat4 u_view;
uniform mat4 u_proj;

uniform sampler2D s_color;
uniform sampler2D s_depth;

in vec2 v_tex;
out vec4 rt_color;
out float rt_depth;

void main() {
    //get head position to fix it
    //TODO: do most of the matrix multiplications outside of the fragment shader (so it wont be calculated for every single pixel, even though it's always the same)
    vec4 head = (u_view_context_l * vec4(0.0, 0.0, 0.0, 1.0) + u_view_context_r * vec4(0.0, 0.0, 0.0, 1.0)) / 2.0;
    mat4 translate_head = mat4(1.0, 0.0, 0.0, 0.0,  0.0, 1.0, 0.0, 0.0,  0.0, 0.0, 1.0, 0.0, -head.x, -head.y, -head.z, 1.0);

    vec4 ndc = vec4(v_tex * 2.0 - 1.0, 0.9, 1.0);
    vec4 world_dir = inverse(u_proj * (translate_head*u_view)) * ndc;
    world_dir.xyz = normalize(world_dir.xyz)/world_dir.w;
    vec2 tex = vec2(atan(world_dir.z, world_dir.x) + PI, acos(-world_dir.y)) / vec2(2.0 * PI, PI);

    rt_color = vec4(texture(s_color, tex).rgb, 1.0);
    rt_depth = vec4(texture(s_depth, tex)).r;
}
