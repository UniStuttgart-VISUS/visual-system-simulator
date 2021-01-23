const float PI = 3.1415926535897932384626433832795;

uniform uint u_flags;
uniform mat4 u_head;
uniform vec2 u_fov;
uniform mat4 u_proj_view;

uniform sampler2D s_rgb;

in vec2 v_tex;
out vec4 rt_color;
out float rt_depth;
out vec4 rt_deflection;
out vec4 rt_color_change;
out vec4 rt_color_uncertainty;
out vec4 rt_covariances;


void main() {
    float depth = 0.5;
    vec2 tex = v_tex;
         
    if ((u_flags & 1) == 1) {
        // Equirectangular 360° projection.
        if((u_flags & 8) == 8){
            // Use VR projection.
            vec4 ndc = vec4(v_tex * 2.0 - 1.0, 0.9, 1.0);
            vec4 world_dir = inverse(u_proj_view) * ndc;
            world_dir.xyz = normalize(world_dir.xyz)/world_dir.w;
            tex = vec2(atan(world_dir.z, world_dir.x) + PI, acos(-world_dir.y)) / vec2(2.0 * PI, PI);
        }else{
            vec2 ndc = tex * 2.0 - 1.0;
            vec4 cam_dir = vec4(normalize(vec3(ndc * vec2(tan(0.5 * u_fov.x), tan(0.5 * u_fov.y)), 1.0)), 1.0);
            vec4 rd = u_head * cam_dir;
            tex = vec2(atan(rd.z, rd.x) + PI, acos(-rd.y)) / vec2(2.0 * PI, PI);
        }
    }

    if ((u_flags & 2) == 2) {
        // Vertically flipped.
        tex = vec2(tex.x, 1.0 - tex.y);
    }

    if ((u_flags & 4) == 4) {
        // RGBD horizontally splitted.
        vec2 tex_depth = vec2(tex.x, 0.5 * tex.y);
        tex = vec2(tex.x, 0.5 * tex.y + 0.5);
        depth = texture(s_rgb, tex_depth).r;
    }

    rt_color = vec4(texture(s_rgb, tex).rgb, 1.0);
    rt_depth = depth;

    rt_deflection =         vec4(0.0);
    rt_color_change =       vec4(0.0);
    rt_color_uncertainty =  vec4(0.0);
    rt_covariances =        vec4(0.0);
}
