use super::*;

struct Uniforms{
    format: i32,
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

    pipeline: wgpu::RenderPipeline,
    uniforms: ShaderUniforms<Uniforms>,
    sources_bind_group: wgpu::BindGroup,
    targets: ColorTargets,

    //TODO WGPU: texture_y: Option<gfx::handle::Texture<Resources, gfx::format::R8>>,
    //TODO WGPU: texture_u: Option<gfx::handle::Texture<Resources, gfx::format::R8>>,
    //TODO WGPU: texture_v: Option<gfx::handle::Texture<Resources, gfx::format::R8>>,

    //TODO WGPU: pso: gfx::PipelineState<Resources, pipe::Meta>,
    //TODO WGPU:  pso_data: pipe::Data<Resources>,
}

impl UploadYuvBuffer {
    pub fn is_empty(&self) -> bool {
        self.buffer_next.is_none()
    }

    pub fn upload_buffer(&mut self, buffer: YuvBuffer) {
        // Test if we have to invalidate textures.
        //TODO WGPU:if let Some(texture_y) = &self.texture_y {
        //TODO WGPU:    let info_y = texture_y.get_info().to_image_info(0);
         //TODO WGPU:   if buffer.width != info_y.width as u32 || buffer.height != info_y.height as u32 {
        //TODO WGPU:        self.texture_y = None;
        //TODO WGPU:        self.texture_u = None;
        //TODO WGPU:        self.texture_v = None;
        //TODO WGPU:    }
        //TODO WGPU:}

        self.buffer_next = Some(buffer);
    }

    pub fn set_format(&mut self, format: YuvFormat) {
        //TODO WGPU:self.uniforms.format = format as i32;
    }
}

impl Node for UploadYuvBuffer {
    fn new(surface: &Surface) -> Self {
        let device = surface.device().borrow_mut();
        let queue = surface.queue().borrow_mut();

        let uniforms = ShaderUniforms::new(&device, 
            Uniforms {
                format: 0,
            });

        let (sources_bind_group_layout, sources_bind_group) = create_color_sources_bind_group(&device, &queue, "UploadYuvBuffer");

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("UploadYuvBuffer Shader"),
            source: wgpu::ShaderSource::Wgsl(concat!(
                include_str!("../common.wgsl"),
                include_str!("../vert.wgsl"),
                include_str!("upload.wgsl")).into()),
        });

        let pipeline = create_render_pipeline(
            &device,
            &[&shader, &shader],
            &["vs_main", "fs_main"],
            &[&uniforms.bind_group_layout, &sources_bind_group_layout],
            &all_color_states(),
            None,
            Some("UploadYuvBuffer Render Pipeline"));

        UploadYuvBuffer {
            buffer_next: None,
            pipeline,
            uniforms,
             sources_bind_group, 
             targets: ColorTargets::new(&device, "UploadYuvBuffer"),
        //TODO WGPU:    texture_y: None,
        //TODO WGPU:    texture_u: None,
        //TODO WGPU:    texture_v: None,

        //TODO WGPU:    pso,
        //TODO WGPU:    pso_data: pipe::Data {
        //TODO WGPU:        u_format: YuvFormat::YCbCr as i32,
        //TODO WGPU:        s_y: (srv.clone(), sampler.clone()),
        //TODO WGPU:        s_u: (srv.clone(), sampler.clone()),
        //TODO WGPU:        s_v: (srv, sampler),
        //TODO WGPU:        rt_color: rtv,
        //TODO WGPU:    },
        }
    }

    fn negociate_slots(&mut self, surface: &Surface, slots: NodeSlots) -> NodeSlots {
        // if let Some(buffer) = &self.buffer_next {
        //     let mut factory = window.factory().borrow_mut();

        //     let (texture_y, view_y) = load_single_channel_texture_from_bytes(
        //         &mut factory,
        //         &buffer.pixels_y,
        //         buffer.width as u32,
        //         buffer.height as u32,
        //     )
        //     .unwrap();
        //     let (texture_u, view_u) = load_single_channel_texture_from_bytes(
        //         &mut factory,
        //         &buffer.pixels_u,
        //         (buffer.width / 2) as u32,
        //         (buffer.height / 2) as u32,
        //     )
        //     .unwrap();
        //     let (texture_v, view_v) = load_single_channel_texture_from_bytes(
        //         &mut factory,
        //         &buffer.pixels_v,
        //         (buffer.width / 2) as u32,
        //         (buffer.height / 2) as u32,
        //     )
        //     .unwrap();

        //     self.texture_y = Some(texture_y);
        //     self.texture_u = Some(texture_u);
        //     self.texture_v = Some(texture_v);

        //     let sampler = factory.create_sampler_linear();
        //     self.pso_data.s_y = (view_y, sampler.clone());
        //     self.pso_data.s_u = (view_u, sampler.clone());
        //     self.pso_data.s_v = (view_v, sampler);
        // }

        // let mut width = 1;
        // let mut height = 1;
        // if let Some(texture_y) = &self.texture_y {
        //     let info = texture_y.get_info().to_image_info(0);
        //     width = info.width as u32;
        //     height = info.height as u32;
        // }

        // let slots = slots.emplace_color_output(window, width, height);
        // self.pso_data.rt_color = slots.as_color();

        slots
    }

    fn render(&mut self, surface: &Surface, encoder: &mut CommandEncoder, screen: Option<&RenderTexture>) {

        // if let Some(texture_y) = &self.texture_y {
        //     if let Some(texture_u) = &self.texture_u {
        //         if let Some(texture_v) = &self.texture_v {
        //             if let Some(buffer) = self.buffer_next.take() {
        //                 // Update texture pixels.
        //                 let size = [buffer.width as u16, buffer.height as u16];
        //                 let half_size = [(buffer.width / 2) as u16, (buffer.height / 2) as u16];
        //                 let offset = [0, 0];
        //                 update_single_channel_texture(
        //                     &mut encoder,
        //                     &texture_y,
        //                     size,
        //                     offset,
        //                     &buffer.pixels_y,
        //                 );
        //                 update_single_channel_texture(
        //                     &mut encoder,
        //                     &texture_u,
        //                     half_size,
        //                     offset,
        //                     &buffer.pixels_u,
        //                 );
        //                 update_single_channel_texture(
        //                     &mut encoder,
        //                     &texture_v,
        //                     half_size,
        //                     offset,
        //                     &buffer.pixels_v,
        //                 );
        //             }
        //         }
        //     }
        // }

        // encoder.draw(&gfx::Slice::from_vertex_count(6), &self.pso, &self.pso_data);
    }
}
