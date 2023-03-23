use super::*;

struct Uniforms {
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

    texture_y: Option<Texture>,
    texture_u: Option<Texture>,
    texture_v: Option<Texture>,
}

impl UploadYuvBuffer {
    pub fn new(surface: &Surface) -> Self {
        let device = surface.device().borrow_mut();
        let queue = surface.queue().borrow_mut();

        let uniforms = ShaderUniforms::new(
            &device,
            Uniforms {
                format: YuvFormat::YCbCr as i32,
            },
        );

        let (sources_bind_group_layout, sources_bind_group) = create_textures_bind_group(
            &device,
            &[
                &placeholder_texture(&device, &queue, Some("UploadYuvBuffer in_y (placeholder)"))
                    .unwrap(),
                &placeholder_texture(&device, &queue, Some("UploadYuvBuffer in_u (placeholder)"))
                    .unwrap(),
                &placeholder_texture(&device, &queue, Some("UploadYuvBuffer in_v (placeholder)"))
                    .unwrap(),
            ],
        );

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("UploadYuvBuffer Shader"),
            source: wgpu::ShaderSource::Wgsl(
                concat!(include_str!("../vert.wgsl"), include_str!("upload.wgsl")).into(),
            ),
        });

        let pipeline = create_render_pipeline(
            &device,
            &[&shader, &shader],
            &["vs_main", "fs_main"],
            &[&uniforms.bind_group_layout, &sources_bind_group_layout],
            &all_color_states(),
            None,
            Some("UploadYuvBuffer Render Pipeline"),
        );

        UploadYuvBuffer {
            buffer_next: None,
            pipeline,
            uniforms,
            sources_bind_group,
            targets: ColorTargets::new(&device, "UploadYuvBuffer"),
            texture_y: None,
            texture_u: None,
            texture_v: None,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.buffer_next.is_none()
    }

    pub fn upload_buffer(&mut self, buffer: YuvBuffer) {
        // Test if we have to invalidate textures.
        if let Some(texture_y) = &self.texture_y {
            if buffer.width != texture_y.width as u32 || buffer.height != texture_y.height as u32 {
                self.texture_y = None;
                self.texture_u = None;
                self.texture_v = None;
            }
        }

        self.buffer_next = Some(buffer);
    }

    pub fn set_format(&mut self, format: YuvFormat) {
        self.uniforms.data.format = format as i32;
    }
}

impl Node for UploadYuvBuffer {
    fn negociate_slots(&mut self, surface: &Surface, slots: NodeSlots) -> NodeSlots {
        if let Some(buffer) = &self.buffer_next {
            let device = surface.device().borrow_mut();
            let queue = surface.queue().borrow_mut();

            let texture_y = load_texture_from_bytes(
                &device,
                &queue,
                &buffer.pixels_y,
                buffer.width as u32,
                buffer.height as u32,
                create_sampler_linear(&device),
                wgpu::TextureFormat::R8Unorm,
                Some("UploadYuvBuffer in_y"),
            )
            .unwrap();
            let texture_u = load_texture_from_bytes(
                &device,
                &queue,
                &buffer.pixels_u,
                (buffer.width / 2) as u32,
                (buffer.height / 2) as u32,
                create_sampler_linear(&device),
                wgpu::TextureFormat::R8Unorm,
                Some("UploadYuvBuffer in_u"),
            )
            .unwrap();
            let texture_v = load_texture_from_bytes(
                &device,
                &queue,
                &buffer.pixels_v,
                (buffer.width / 2) as u32,
                (buffer.height / 2) as u32,
                create_sampler_linear(&device),
                wgpu::TextureFormat::R8Unorm,
                Some("UploadYuvBuffer in_v"),
            )
            .unwrap();

            (_, self.sources_bind_group) =
                create_textures_bind_group(&device, &[&texture_y, &texture_u, &texture_v]);

            self.texture_y = Some(texture_y);
            self.texture_u = Some(texture_u);
            self.texture_v = Some(texture_v);
        }

        let mut width = 1;
        let mut height = 1;
        if let Some(texture_y) = &self.texture_y {
            width = texture_y.width as u32;
            height = texture_y.height as u32;
        }

        let slots = slots.emplace_color_output(surface, width, height, "UploadYuvBuffer");
        self.targets = slots.as_all_colors_target();

        slots
    }

    fn render(
        &mut self,
        surface: &Surface,
        encoder: &mut CommandEncoder,
        screen: Option<&RenderTexture>,
    ) {
        let queue = surface.queue().borrow_mut();
        self.uniforms.update(&queue);

        if let (Some(texture_y), Some(texture_u), Some(texture_v)) =
            (&self.texture_y, &self.texture_u, &self.texture_v)
        {
            if let Some(buffer) = self.buffer_next.take() {
                // Update texture pixels.
                let size = [buffer.width as u32, buffer.height as u32];
                let half_size = [(buffer.width / 2) as u32, (buffer.height / 2) as u32];
                update_texture(&queue, &texture_y, size, &buffer.pixels_y);
                update_texture(&queue, &texture_u, half_size, &buffer.pixels_u);
                update_texture(&queue, &texture_v, half_size, &buffer.pixels_v);
            }
        }

        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("UploadYuvBuffer render_pass"),
            color_attachments: &self.targets.color_attachments(screen),
            depth_stencil_attachment: None,
        });

        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, &self.uniforms.bind_group, &[]);
        render_pass.set_bind_group(1, &self.sources_bind_group, &[]);
        render_pass.draw(0..6, 0..1);
    }
}
