use super::*;
use wgpu::{util::DeviceExt, CommandEncoder};

// gfx_defines! {
//     pipeline pipe {
//         s_source: gfx::TextureSampler<[f32; 4]> = "s_color",
//         rt_color: gfx::RenderTarget<ColorFormat> = "rt_color",
//     }
// }

struct Uniforms{
    test_color: [f32; 4],
}

struct Sources{
    s_rgb: texture::Texture,
    s_rgb_bind_group: wgpu::BindGroup,
}

struct Targets{
    rt_color: RenderTexture,
}

pub struct TestNode {
    pipeline: wgpu::RenderPipeline,
    uniforms: ShaderUniforms<Uniforms>,
    sources: Sources,
    targets: Targets,
}

impl Node for TestNode {
    fn new(window: &window::Window) -> Self {
        let device = window.device().borrow_mut();
        let queue = window.queue().borrow_mut();

        let uniforms = ShaderUniforms::new(&device, Uniforms{test_color: [1.0, 1.0, 1.0, 1.0]});

        let s_rgb = load_texture_from_bytes(&device, &queue, &[255; 4], 1, 1, Some("TestNode s_rgb")).unwrap();
        let sampler = create_sampler_linear(&device).unwrap();
        let (s_rgb_bind_group_layout, s_rgb_bind_group) = s_rgb.create_bind_group(&device, &sampler);

        let rt_color = create_texture_render_target(&device, 1, 1, ColorFormat, Some("TestNode rt_color"));

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("TestNode Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("mod.wgsl").into()),
        });

        let pipeline = create_render_pipeline(
            &device,
            &[&shader, &shader],
            &["vs_main", "fs_main"],
            &[&s_rgb_bind_group_layout, &uniforms.bind_group_layout],
            &[blended_color_state(ColorFormat)],
            None,
            Some("TestNode Render Pipeline"));

        TestNode {
            pipeline,
            uniforms,
            sources: Sources{
                s_rgb,
                s_rgb_bind_group,
            },
            targets: Targets{
                rt_color,
            },
        }
    }

    fn negociate_slots(&mut self, window: &window::Window, slots: NodeSlots) -> NodeSlots {
        let slots = slots.to_color_input(window).to_color_output(window);
        (self.sources.s_rgb, self.sources.s_rgb_bind_group) = slots.as_color_source(&(window.device().borrow_mut()));
        self.targets.rt_color = slots.as_color_target();

        slots
    }

    fn render(&mut self, window: &window::Window, encoder: &mut CommandEncoder, screen: &RenderTexture) {
        let queue = window.queue().borrow_mut();
        self.uniforms.data.test_color = [1.0, 0.1, 0.1, 1.0];
        self.uniforms.update(&queue);

        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("testnode_render_pass"),
            color_attachments: &[screen.to_color_attachment()],
            depth_stencil_attachment: None,
        });

        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, &self.sources.s_rgb_bind_group, &[]);
        render_pass.set_bind_group(1, &self.uniforms.bind_group, &[]);
        render_pass.draw(0..6, 0..1);
    }
}
