#[derive(Clone, Debug)]
pub struct View {
    pub view_index: usize,
    pub eye_index: usize,
    pub viewport: Viewport,
    pub view: cgmath::Matrix4<f32>,
    pub projection: cgmath::Matrix4<f32>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Viewport {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}
