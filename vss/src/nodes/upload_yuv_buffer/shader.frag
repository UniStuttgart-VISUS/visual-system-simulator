uniform int u_format;

uniform sampler2D s_y;
uniform sampler2D s_u;
uniform sampler2D s_v;

in vec2 v_tex;
out vec4 rt_color;

void main() {
    if (u_format == 0) {
        // YUV/YCbCr to RGB color space conversion.
        // Assuming the following input:
        //   - Y texture has size `width * height`
        //   - Y and U are vertically flipped
        //   - U and V textures have size `(width / 2) * (height / 2)`
        vec2 v_tex_flip = vec2(v_tex.x, 1.0 - v_tex.y);
        vec3 yuv = vec3(texture(s_y, v_tex_flip).r,
            texture(s_u, v_tex_flip).r - 0.5,
            texture(s_v, v_tex_flip).r - 0.5);

        rt_color = vec4(yuv.x + yuv.z * 1.4,
            yuv.x + yuv.y * -0.343 + yuv.z * -0.711,
            yuv.x + yuv.y * 1.765, 1.0);
    } else if (u_format == 1) {
        // YUV_420_888 to RGB color space conversion.
        // The textures are the original android-cam textures, formatted like so:
        // every pixel has a corresponding y value and shares a u and v with its neighbour (only one direction)
        // name: [ r,  g,  b,  a], [ r,  g,  b,  a] ... (where [] represents a pixel)
        //    y: [y1, y2, y3, y4], [y5, y6, y7, y8] ...
        //    u: [u1, v2, u3, v4], [u5, v6, u7, v8] ...
        //    v: [v1, u2, v3, u4], [v5, u6, v7, u8] ...
        ivec2 v_tex_i = ivec2(0,0); //XXX ivec2(v_tex * vec2(textureSize(s_u)));
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
    } else {
        rt_color = vec4(1.0, 0.0, 1.0, 1.0);
    }
}
