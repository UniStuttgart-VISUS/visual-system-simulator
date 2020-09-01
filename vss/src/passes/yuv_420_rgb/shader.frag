uniform vec2 u_resolution_in;
uniform sampler2D s_y;
uniform sampler2D s_u;
uniform sampler2D s_v;

in vec2 v_tex;
out vec4 rt_color;

// YUV_420_888
// The textures are the original android-cam textures, formatted like so:
// every pixel has a corresponding y value and shares a u and v with its neighbour (only one direction)
// name: [ r,  g,  b,  a], [ r,  g,  b,  a] ... (where [] represents a pixel)
//    y: [y1, y2, y3, y4], [y5, y6, y7, y8] ...
//    u: [u1, v2, u3, v4], [u5, v6, u7, v8] ...
//    v: [v1, u2, v3, u4], [v5, u6, v7, u8] ...
void main() {
    ivec2 v_tex_i = ivec2(v_tex * u_resolution_in);
    v_tex_i.y /= 2;

    float y = texture(s_y, v_tex).r;
    float u = texelFetch(s_u, v_tex_i, 0).r;
    float v = texelFetch(s_v, v_tex_i, 0).r;

    if ((v_tex_i.x) % 2 == 1) {
        float temp = u;
        u = v;
        v = temp;
    }

    vec4 yuva = vec4(y, (u - 0.5), (v - 0.5), 1.0);

    vec4 rgba = vec4(0.0);
    rgba.r = yuva.x * 1.0 + yuva.y * 0.0 + yuva.z * 1.4;
    rgba.g = yuva.x * 1.0 + yuva.y * -0.343 + yuva.z * -0.711;
    rgba.b = yuva.x * 1.0 + yuva.y * 1.765 + yuva.z * 0.0;
    rgba.a = 1.0;

    rt_color = rgba;
}
