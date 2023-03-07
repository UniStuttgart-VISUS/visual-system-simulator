use super::*;
use gfx;

gfx_defines! {
    pipeline pipe {
        u_format: gfx::Global<i32> = "u_format",
        s_y: gfx::TextureSampler<f32> = "s_y",
        s_u: gfx::TextureSampler<f32> = "s_u",
        s_v: gfx::TextureSampler<f32> = "s_v",
        rt_color: gfx::RenderTarget<ColorFormat> = "rt_color",
    }
}

pub enum YuvFormat {
    YCbCr = 0,
    _420888 = 1,
}

// A buffer representing color information.
//
// For YUV, the U anv C channels only have half width and height by convetion.
pub struct YuvBuffer {
    pub pixels_y: Box<[u8]>,
    pub pixels_u: Box<[u8]>,
    pub pixels_v: Box<[u8]>,
    pub width: u32,
    pub height: u32,
}

pub struct UploadYuvBuffer {
    buffer_next: Option<YuvBuffer>,
    texture_y: Option<gfx::handle::Texture<Resources, gfx::format::R8>>,
    texture_u: Option<gfx::handle::Texture<Resources, gfx::format::R8>>,
    texture_v: Option<gfx::handle::Texture<Resources, gfx::format::R8>>,

    pso: gfx::PipelineState<Resources, pipe::Meta>,
    pso_data: pipe::Data<Resources>,
}

impl UploadYuvBuffer {
    pub fn is_empty(&self) -> bool {
        self.buffer_next.is_none()
    }

    pub fn upload_buffer(&mut self, buffer: YuvBuffer) {
        // Test if we have to invalidate textures.
        if let Some(texture_y) = &self.texture_y {
            let info_y = texture_y.get_info().to_image_info(0);
            if buffer.width != info_y.width as u32 || buffer.height != info_y.height as u32 {
                self.texture_y = None;
                self.texture_u = None;
                self.texture_v = None;
            }
        }

        self.buffer_next = Some(buffer);
    }

    pub fn set_format(&mut self, format: YuvFormat) {
        self.pso_data.u_format = format as i32;
    }
}

impl Node for UploadYuvBuffer {
    fn new(surface: &Surface) -> Self {
        let mut factory = window.factory().borrow_mut();

        let pso = factory
            .create_pipeline_simple(
                &include_glsl!("../mod.vert"),
                &include_glsl!("upload.frag"),
                pipe::new(),
            )
            .unwrap();

        let sampler = factory.create_sampler_linear();
        let (_, srv, _): (
            _,
            _,
            gfx::handle::RenderTargetView<gfx_device_gl::Resources, f32>,
        ) = factory.create_render_target(1, 1).unwrap();
        let (_, _, rtv) = factory.create_render_target(1, 1).unwrap();

        UploadYuvBuffer {
            buffer_next: None,
            texture_y: None,
            texture_u: None,
            texture_v: None,

            pso,
            pso_data: pipe::Data {
                u_format: YuvFormat::YCbCr as i32,
                s_y: (srv.clone(), sampler.clone()),
                s_u: (srv.clone(), sampler.clone()),
                s_v: (srv, sampler),
                rt_color: rtv,
            },
        }
    }

    fn negociate_slots(&mut self, surface: &Surface, slots: NodeSlots) -> NodeSlots {
        if let Some(buffer) = &self.buffer_next {
            let mut factory = window.factory().borrow_mut();

            let (texture_y, view_y) = load_single_channel_texture_from_bytes(
                &mut factory,
                &buffer.pixels_y,
                buffer.width as u32,
                buffer.height as u32,
            )
            .unwrap();
            let (texture_u, view_u) = load_single_channel_texture_from_bytes(
                &mut factory,
                &buffer.pixels_u,
                (buffer.width / 2) as u32,
                (buffer.height / 2) as u32,
            )
            .unwrap();
            let (texture_v, view_v) = load_single_channel_texture_from_bytes(
                &mut factory,
                &buffer.pixels_v,
                (buffer.width / 2) as u32,
                (buffer.height / 2) as u32,
            )
            .unwrap();

            self.texture_y = Some(texture_y);
            self.texture_u = Some(texture_u);
            self.texture_v = Some(texture_v);

            let sampler = factory.create_sampler_linear();
            self.pso_data.s_y = (view_y, sampler.clone());
            self.pso_data.s_u = (view_u, sampler.clone());
            self.pso_data.s_v = (view_v, sampler);
        }

        let mut width = 1;
        let mut height = 1;
        if let Some(texture_y) = &self.texture_y {
            let info = texture_y.get_info().to_image_info(0);
            width = info.width as u32;
            height = info.height as u32;
        }

        let slots = slots.emplace_color_output(window, width, height);
        self.pso_data.rt_color = slots.as_color();

        slots
    }

    fn render(&mut self, surface: &Surface) {
        let mut encoder = window.encoder().borrow_mut();

        if let Some(texture_y) = &self.texture_y {
            if let Some(texture_u) = &self.texture_u {
                if let Some(texture_v) = &self.texture_v {
                    if let Some(buffer) = self.buffer_next.take() {
                        // Update texture pixels.
                        let size = [buffer.width as u16, buffer.height as u16];
                        let half_size = [(buffer.width / 2) as u16, (buffer.height / 2) as u16];
                        let offset = [0, 0];
                        update_single_channel_texture(
                            &mut encoder,
                            &texture_y,
                            size,
                            offset,
                            &buffer.pixels_y,
                        );
                        update_single_channel_texture(
                            &mut encoder,
                            &texture_u,
                            half_size,
                            offset,
                            &buffer.pixels_u,
                        );
                        update_single_channel_texture(
                            &mut encoder,
                            &texture_v,
                            half_size,
                            offset,
                            &buffer.pixels_v,
                        );
                    }
                }
            }
        }

        encoder.draw(&gfx::Slice::from_vertex_count(6), &self.pso, &self.pso_data);
    }
}
