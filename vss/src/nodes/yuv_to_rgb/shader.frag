uniform sampler2D s_y;
uniform sampler2D s_u;
uniform sampler2D s_v;

in vec2 v_tex;
out vec4 rt_color;

// YUV/YCbCr to RGB color space conversion.
//
// Assuming the following input:
//   - Y,U, and are vertically flipped
//   - Y texture with size (width * height)
//   - U and V textures with size ((width / 2) * (height / 2))
void main() {
    vec2 v_tex_flip = vec2(v_tex.x, 1.0 - v_tex.y);
    vec3 yuv = vec3(texture(s_y, v_tex_flip).r,
        texture(s_u, v_tex_flip).r - 0.5,
        texture(s_v, v_tex_flip).r - 0.5);

    rt_color = vec4(yuv.x + yuv.z * 1.4,
        yuv.x + yuv.y * -0.343 + yuv.z * -0.711,
        yuv.x + yuv.y * 1.765, 1.0);
}
