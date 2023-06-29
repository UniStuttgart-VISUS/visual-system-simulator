use super::*;
use wgpu::Origin3d;

struct Uniforms {
    format: i32,
}

#[derive(Copy, Clone)]
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
    format: YuvFormat,

    pipeline: wgpu::RenderPipeline,
    uniforms: ShaderUniforms<Uniforms>,
    sources_bind_group: wgpu::BindGroup,
    targets: ColorDepthTargets,

    texture_y: Option<Texture>,
    texture_u: Option<Texture>,
    texture_v: Option<Texture>,
}

impl UploadYuvBuffer {
    pub fn new(surface: &Surface) -> Self {
        let device = surface.device();
        let queue = surface.queue();

        let uniforms = ShaderUniforms::new(
            device,
            Uniforms {
                format: YuvFormat::YCbCr as i32,
            },
        );

        let (sources_bind_group_layout, sources_bind_group) = create_textures_bind_group(
            device,
            &[
                &placeholder_single_channel_texture(
                    device,
                    queue,
                    Some("UploadYuvBuffer in_y (placeholder)"),
                )
                .unwrap(),
                &placeholder_single_channel_texture(
                    device,
                    queue,
                    Some("UploadYuvBuffer in_u (placeholder)"),
                )
                .unwrap(),
                &placeholder_single_channel_texture(
                    device,
                    queue,
                    Some("UploadYuvBuffer in_v (placeholder)"),
                )
                .unwrap(),
            ],
        );

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("UploadYuvBuffer Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("upload.wgsl").into()),
        });

        let pipeline = create_render_pipeline(
            device,
            &[&shader, &shader],
            &["vs_main", "fs_main"],
            &[&uniforms.bind_group_layout, &sources_bind_group_layout],
            &all_color_states(),
            None,
            Some("UploadYuvBuffer Render Pipeline"),
        );

        UploadYuvBuffer {
            buffer_next: None,
            format: YuvFormat::YCbCr,
            pipeline,
            uniforms,
            sources_bind_group,
            targets: ColorDepthTargets::new(device, "UploadYuvBuffer"),
            texture_y: None,
            texture_u: None,
            texture_v: None,
        }
    }

    pub fn get_formatted_sizes(
        format: YuvFormat,
        width: u32,
        height: u32,
    ) -> ([u32; 2], [u32; 2], [u32; 2]) {
        match format {
            YuvFormat::YCbCr => (
                [width, height],
                [width / 2, height / 2],
                [width / 2, height / 2],
            ),
            YuvFormat::_420888 => ([width, height], [width, height / 2], [1, 1]),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.buffer_next.is_none()
    }

    pub fn upload_buffer(&mut self, buffer: YuvBuffer) {
        // Test if we have to invalidate textures.
        if let Some(texture_y) = &self.texture_y {
            if buffer.width != texture_y.width || buffer.height != texture_y.height {
                self.texture_y = None;
                self.texture_u = None;
                self.texture_v = None;
            }
        }

        self.buffer_next = Some(buffer);
    }

    pub fn set_format(&mut self, format: YuvFormat) {
        self.format = format;
        self.uniforms.data.format = format as i32;
    }
}

impl Node for UploadYuvBuffer {
    fn name(&self) -> &'static str {
        "UploadYuvBuffer"
    }

    fn negociate_slots(
        &mut self,
        surface: &Surface,
        slots: NodeSlots,
        original_image: &mut Option<Texture>,
    ) -> NodeSlots {
        if let Some(buffer) = &self.buffer_next {
            let device = surface.device();
            let queue = surface.queue();
            let (size_y, size_u, size_v) =
                UploadYuvBuffer::get_formatted_sizes(self.format, buffer.width, buffer.height);

            let texture_y = load_texture_from_bytes(
                device,
                queue,
                &vec![0; (size_y[0] * size_y[1] * 4) as usize],
                size_y[0],
                size_y[1],
                create_sampler_linear(device),
                wgpu::TextureFormat::R8Unorm,
                Some("UploadYuvBuffer in_y"),
            )
            .unwrap();
            let texture_u = load_texture_from_bytes(
                device,
                queue,
                &vec![0; (size_u[0] * size_u[1] * 4) as usize],
                size_u[0],
                size_u[1],
                create_sampler_linear(device),
                wgpu::TextureFormat::R8Unorm,
                Some("UploadYuvBuffer in_u"),
            )
            .unwrap();
            let texture_v = load_texture_from_bytes(
                device,
                queue,
                &vec![0; (size_v[0] * size_v[1] * 4) as usize],
                size_v[0],
                size_v[1],
                create_sampler_linear(device),
                wgpu::TextureFormat::R8Unorm,
                Some("UploadYuvBuffer in_v"),
            )
            .unwrap();

            (_, self.sources_bind_group) =
                create_textures_bind_group(device, &[&texture_y, &texture_u, &texture_v]);

            self.texture_y = Some(texture_y);
            self.texture_u = Some(texture_u);
            self.texture_v = Some(texture_v);
        }

        let mut width = 1;
        let mut height = 1;
        if let Some(texture_y) = &self.texture_y {
            width = texture_y.width;
            height = texture_y.height;
        }

        let slots = slots.emplace_color_depth_output(surface, height, width, "UploadYuvBuffer");
        self.targets = slots.as_all_target();

        let (color_out, _) = slots.as_color_depth_target();
        original_image.replace(color_out.as_texture());

        slots
    }

    fn render(
        &mut self,
        surface: &Surface,
        encoder: &mut CommandEncoder,
        screen: Option<&RenderTexture>,
    ) {
        let queue = surface.queue();
        self.uniforms.upload(queue);

        if let (Some(texture_y), Some(texture_u), Some(texture_v)) =
            (&self.texture_y, &self.texture_u, &self.texture_v)
        {
            if let Some(buffer) = self.buffer_next.take() {
                // Update texture pixels.
                let (size_y, size_u, size_v) =
                    UploadYuvBuffer::get_formatted_sizes(self.format, buffer.width, buffer.height);
                match self.format {
                    YuvFormat::YCbCr => {
                        update_texture(queue, texture_y, size_y, None, &buffer.pixels_y, 0);
                        update_texture(queue, texture_u, size_u, None, &buffer.pixels_u, 0);
                        update_texture(queue, texture_v, size_v, None, &buffer.pixels_v, 0);
                    }
                    YuvFormat::_420888 => {
                        update_texture(queue, texture_y, size_y, None, &buffer.pixels_y, 0);
                        update_texture(
                            queue,
                            texture_u,
                            [size_u[0], size_u[1] / 2],
                            None,
                            &buffer.pixels_u,
                            0,
                        );
                        update_texture(
                            queue,
                            texture_u,
                            [size_u[0], size_u[1] / 2],
                            Some(Origin3d {
                                x: 0,
                                y: size_u[1] / 2,
                                z: 0,
                            }),
                            &buffer.pixels_v,
                            (size_u[0] * size_u[1] / 2 - 1) as u64,
                        );
                    }
                }
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
