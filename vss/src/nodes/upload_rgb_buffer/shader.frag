
const float PI = 3.1415926535897932384626433832795;

uniform int u_mode;
uniform mat4 u_head;
uniform vec2 u_fov;

uniform sampler2D s_rgb;

in vec2 v_tex;
out vec4 rt_color;

vec3 rotateXY(vec3 p, vec2 angle) {
    vec2 c = cos(angle), s = sin(angle);
    p = vec3(p.x, c.x*p.y + s.x*p.z, -s.x*p.y + c.x*p.z);
    return vec3(c.y*p.x + s.y*p.z, p.y, -s.y*p.x + c.y*p.z);
}

void main() {
    if (u_mode == 0) {
        // Pass through.
        rt_color = vec4(texture(s_rgb, v_tex).rgb, 1.0);
    } else if (u_mode == 1) {
        // Vertically flipped.
        vec2 v_tex_flipped = vec2(v_tex.x, 1.0 - v_tex.y);
        rt_color = vec4(texture(s_rgb, v_tex_flipped).rgb, 1.0);
    } else if (u_mode == 2) {
        // Equirectangular 360° projection.
        vec2 ndc = v_tex.xy * 2.0 - 1.0;
        vec4 cam_dir = vec4(normalize(vec3(ndc * vec2(tan(0.5 * u_fov.x), tan(0.5 * u_fov.y)), 1.0)), 1.0);
        vec4 rd = u_head * cam_dir;
        vec2 v_tex_er = vec2(atan(rd.z, rd.x) + PI, acos(-rd.y)) / vec2(2.0 * PI, PI);
        rt_color = vec4(texture(s_rgb, v_tex_er).rgb, 1.0);
    }
}
