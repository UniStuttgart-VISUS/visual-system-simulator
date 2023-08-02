use super::*;
use std::iter;

pub struct NormalMapGenerator {
    pub texture: RenderTexture,
    pipeline: wgpu::RenderPipeline,
}

impl NormalMapGenerator {
    pub fn new(surface: &Surface) -> Self {
        let device = surface.device();

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Generator Shader"),
            source: wgpu::ShaderSource::Wgsl(
                concat!(
                    include_str!("../vert.wgsl"),
                    include_str!("lens_model.wgsl"),
                    include_str!("generator.wgsl")
                )
                .into(),
            ),
        });

        let texture =
            RenderTexture::empty_highp(device, Some("Generator RenderTexture placeholder"));

        let pipeline = create_render_pipeline(
            device,
            &[&shader, &shader],
            &["vs_main", "fs_main"],
            &[],
            &[simple_color_state(HIGHP_FORMAT)],
            None,
            Some("Generator Render Pipeline"),
        );

        NormalMapGenerator { texture, pipeline }
    }

    pub fn generate(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        width: u32,
        height: u32,
    ) {
        self.texture =
            RenderTexture::create_highp(device, width, height, Some("Generator RenderTexture"));

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Generator Encoder"),
        });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Generator render_pass"),
                color_attachments: &[self.texture.to_color_attachment(Some(CLEAR_COLOR))],
                depth_stencil_attachment: None,
            });
            render_pass.set_pipeline(&self.pipeline);
            render_pass.draw(0..6, 0..1);
        }

        queue.submit(iter::once(encoder.finish()));
    }
}
