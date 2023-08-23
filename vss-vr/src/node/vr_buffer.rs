use vss::*;
use wgpu::CommandEncoder;

/// A node to use existing Textures as input for a flow. Mainly used for vr framebuffers.
pub struct VrBuffer {
    buffer_size: (u32, u32), 
    pipeline: wgpu::RenderPipeline,
    sources_bind_group: wgpu::BindGroup,
    targets: ColorDepthTargets,
}

impl VrBuffer {
    pub fn new(surface: &Surface, color: Texture, depth: Option<Texture>) -> Self {
        let device = surface.device();

        let (sources_bind_group_layout, sources_bind_group) = [
            &color,
            &depth.unwrap_or(
                placeholder_depth_texture(device, Some("VrBuffer s_depth (placeholder)")).unwrap()
            ),
        ]
        .create_bind_group(device);

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("VrBuffer Shader"),
            source: wgpu::ShaderSource::Wgsl(
                concat!(include_str!("../../../vss/src/node/vert.wgsl"), include_str!("vr_buffer.wgsl")).into(),
            ),
        });

        let pipeline = create_render_pipeline(
            device,
            &[&shader, &shader],
            &["vs_main", "fs_main"],
            &[&sources_bind_group_layout],
            &all_color_states(),
            simple_depth_state(DEPTH_FORMAT),
            Some("VrBuffer Render Pipeline"),
        );

        VrBuffer {
            buffer_size: (color.width(), color.height()),
            pipeline,
            sources_bind_group,
            targets: ColorDepthTargets::new(device, "VrBuffer"),
        }
    }

    pub fn set_input_textures(&mut self, surface: &Surface, color: Texture, depth: Option<Texture>) {
        self.buffer_size = (color.width(), color.height());
        let device = surface.device();
        let (_, sources_bind_group) = [
            &color,
            &depth.unwrap_or(
                placeholder_depth_texture(device, Some("VrBuffer s_depth (placeholder)")).unwrap()
            ),
        ]
        .create_bind_group(device);

        self.sources_bind_group = sources_bind_group;

    }
}

impl Node for VrBuffer {
    fn name(&self) -> &'static str {
        "VrBuffer"
    }

    fn negociate_slots(
        &mut self,
        surface: &Surface,
        slots: NodeSlots,
        original_image: &mut Option<Texture>,
    ) -> NodeSlots {
        let (width, height) = self.buffer_size;

        let slots = slots.emplace_color_depth_output(surface, width, height, "VrBuffer");
        self.targets = slots.as_all_target();

        let (color_out, _) = slots.as_color_depth_target();
        original_image.replace(color_out.as_texture());

        slots
    }

    fn render(
        &mut self,
        _surface: &Surface,
        encoder: &mut CommandEncoder,
        screen: Option<&RenderTexture>,
    ) {
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("VrBuffer render_pass"),
            color_attachments: &self.targets.color_attachments(screen),
            depth_stencil_attachment: self.targets.depth_attachment(),
        });

        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, &self.sources_bind_group, &[]);
        render_pass.draw(0..6, 0..1);
    }
}
