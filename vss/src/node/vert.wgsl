
struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
};

// let TRI_POS = array<vec2<f32>, 6>(
//     vec2<f32>(-1.0, -1.0),
//     vec2<f32>(1.0, -1.0),
//     vec2<f32>(1.0, 1.0),
//     vec2<f32>(-1.0, -1.0),
//     vec2<f32>(1.0, 1.0),
//     vec2<f32>(-1.0, 1.0)
// );

// let TRI_TEX = array<vec2<f32>, 6>(
//     vec2<f32>(0.0, 0.0),
//     vec2<f32>(1.0, 0.0),
//     vec2<f32>(1.0, 1.0),
//     vec2<f32>(0.0, 0.0),
//     vec2<f32>(1.0, 1.0),
//     vec2<f32>(0.0, 1.0)
// );

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
    out.tex_coords = pos;
    out.clip_position = vec4<f32>(pos * 2.0 - 1.0, 0.0, 1.0);
    return out;
}
