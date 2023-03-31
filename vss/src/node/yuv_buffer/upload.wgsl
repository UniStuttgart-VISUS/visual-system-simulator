struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
};

@vertex
fn vs_main(
    @builtin(vertex_index) in_vertex_index: u32,
) -> VertexOutput {
    var out: VertexOutput;
    // create two triangles with edges (0,0),(1,0),(1,1) and (1,1),(0,1),(0,0)
    var x = f32((i32(in_vertex_index) % 3) > 0);
    var y = f32((i32(in_vertex_index) % 3) > 1);
    if((i32(in_vertex_index) / 3) > 0){
        x = 1.0-x;
        y = 1.0-y;
    }
    let pos = vec2<f32>(x, y);
    out.tex_coords = vec2<f32>(1.0 - pos.x, pos.y);
    let rot = mat2x2<f32>(vec2<f32>(0.0, 1.0), vec2<f32>(-1.0, 0.0)); // 90° clockwise rotation
    out.clip_position = vec4<f32>(rot * (pos * 2.0 - 1.0), 0.0, 1.0);
    return out;
}

struct Uniforms {
    format: i32
}

@group(0) @binding(0)
var<uniform> uniforms: Uniforms;

@group(1) @binding(0)
var in_y_s: sampler;
@group(1) @binding(1)
var in_y_t: texture_2d<f32>;
@group(1) @binding(2)
var in_u_s: sampler;
@group(1) @binding(3)
var in_u_t: texture_2d<f32>;
@group(1) @binding(4)
var in_v_s: sampler;
@group(1) @binding(5)
var in_v_t: texture_2d<f32>;

struct FragmentOutput {
    @location(0) color: vec4<f32>,
};

@fragment
fn fs_main(in: VertexOutput) -> FragmentOutput {
    var out: FragmentOutput;

    let v_tex = in.tex_coords;

    if (uniforms.format == 0) {
        // YUV/YCbCr to RGB color space conversion using ITU recommendation [BT.709][1].
        // We assume the following input:
        //   - Y channel has full size, i.e., `width * height`
        //   - U and V channels have half size, i.e., `(width / 2) * (height / 2)`
        //   - All channels are vertically flipped
        //
        // [1]: https://www.itu.int/rec/R-REC-BT.709 "BT.709"
        let v_tex_flipped = vec2<f32>(v_tex.x, 1.0 - v_tex.y);
        
        let Y = textureSample(in_y_t, in_y_s, v_tex_flipped).r;
        let Cb = textureSample(in_u_t, in_u_s,  v_tex_flipped).r - 0.5;
        let Cr = textureSample(in_v_t, in_v_s,  v_tex_flipped).r - 0.5;

        // | | [BT.601][1] | [BT.709][2] | [BT.2020][3] |
        // |--|-------|-------|-------|
        // a | 0.299 | 0.2126 | 0.2627 |
        // b | 0.587 | 0.7152 | 0.6780 |
        // c | 0.114 | 0.0722 | 0.0593 |
        // d | 1.772 | 1.8556 | 1.8814 |
        // e | 1.402 | 1.5748 | 1.4747 |
        //
        // [1]: https://www.itu.int/rec/R-REC-BT.601 "BT.601"
        // [2]: https://www.itu.int/rec/R-REC-BT.709 "BT.709"
        // [3]: https://www.itu.int/rec/R-REC-BT.2020 "BT.2020"
        let a: f32 = 0.2126;
        let b: f32 = 0.7152;
        let c: f32 = 0.0722;
        let d: f32 = 1.8556;
        let e: f32 = 1.5748;
        out.color = vec4<f32>(
            Y + e * Cr,
            Y - (a * e / b) * Cr - (c * d / b) * Cb,
            Y + d * Cb,
            1.0);
    } else if (uniforms.format == 1) {
        /* >>>>>>> THE INFORMATION BELOW IS CURRENTLY NOT ACCURATE DUE TO FOLLOWING REASON:
                   the current method assumes NV21 layout instead of the old one (probably either NV12 or Y12)
                   it might be neccessary to include both and make it configurable
                   https://stackoverflow.com/questions/51399908/yuv-420-888-byte-format <<<<<< */
        // YUV_420_888 to RGB color space conversion.
        // The original Android camera textures are formatted like so:
        // every pixel has a corresponding Y value that shares an U and V with its neighbour, in one direction only
        // name: [ r,  g,  b,  a], [ r,  g,  b,  a] ... (where [] represents a pixel)
        //    y: [y1, y2, y3, y4], [y5, y6, y7, y8] ...
        //    u: [u1, v2, u3, v4], [u5, v6, u7, v8] ...
        //    v: [v1, u2, v3, u4], [v5, u6, v7, u8] ...
        var v_tex_i = vec2<i32>(v_tex * vec2<f32>(textureDimensions(in_u_t)));
        v_tex_i.x /= 2; 

        let y = textureSample(in_y_t, in_y_s, v_tex).r;
        var u = textureLoad(in_u_t, vec2<i32>(v_tex_i.x*2, v_tex_i.y), 0).r;
        var v = textureLoad(in_u_t, vec2<i32>(v_tex_i.x*2+1, v_tex_i.y), 0).r;
        // var v = textureLoad(in_v_t, v_tex_i, 0).r;

        // if ((v_tex_i.x) % 2 == 1) {
        //     let temp = u;
        //     u = v;
        //     v = temp;
        // }

        let yuva = vec4<f32>(y, (u - 0.5), (v - 0.5), 1.0);

        var rgba = vec4<f32>(0.0);
        rgba.r = yuva.x * 1.0 + yuva.y * 0.0 + yuva.z * 1.4;
        rgba.g = yuva.x * 1.0 + yuva.y * -0.343 + yuva.z * -0.711;
        rgba.b = yuva.x * 1.0 + yuva.y * 1.765 + yuva.z * 0.0;
        rgba.a = 1.0;

        out.color = rgba;
    } else {
        out.color = vec4(1.0, 0.0, 1.0, 1.0);
    }

    return out;
}
