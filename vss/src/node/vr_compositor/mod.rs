use super::*;

struct Uniforms{
    resolution_out: [f32; 2],
    viewport: [f32; 4],
}

pub struct VRCompositor {
    pipeline: wgpu::RenderPipeline,
    uniforms: ShaderUniforms<Uniforms>,

    source_bind_group: wgpu::BindGroup,
    target: RenderTexture,
}

impl VRCompositor{
    pub fn new(surface: &Surface) -> Self {
        let device = surface.device().borrow_mut();
        let queue = surface.queue().borrow_mut();

        let uniforms = ShaderUniforms::new(&device, 
            Uniforms{
                resolution_out: [1.0, 1.0],
                viewport: [0.0, 0.0, 1.0, 1.0],
            });
        
        let (source_bind_group_layout, source_bind_group) = placeholder_texture(&device, &queue, Some("VR Compositor s_color (placeholder)")).unwrap().create_bind_group(&device);
        let target = placeholder_color_rt(&device, Some("VR Compositor rt_color (placeholder)"));

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("VR Compositor"),
            source: wgpu::ShaderSource::Wgsl(concat!(
                include_str!("mod.wgsl")).into()),
        });

        let pipeline = create_render_pipeline(
            &device,
            &[&shader, &shader],
            &["vs_main", "fs_main"],
            &[&uniforms.bind_group_layout, &source_bind_group_layout],
            &all_color_states(),
            None,
            Some("VR Compositor Render Pipeline"));

        VRCompositor {
            pipeline,
            uniforms,
            source_bind_group,
            target,
        }
    }


    pub fn set_viewport(&mut self, x: f32, y: f32, width: f32, height: f32){
        self.uniforms.data.viewport = [x, y, width, height];
    }
}

impl Node for VRCompositor {
  
    fn negociate_slots(&mut self, surface: &Surface, slots: NodeSlots, _resolution: Option<[u32;2]>, _original_image: &mut Option<Texture>) -> NodeSlots {
        let slots = slots.to_color_input(surface).to_color_output(surface, "VR Compositor Node");

        self.uniforms.data.resolution_out = slots.output_size_f32();

        let device = surface.device().borrow_mut();

        self.source_bind_group = slots.as_color_source(&device).1;
        self.target = slots.as_color_target();

        slots
    }

    fn render(&mut self, surface: &Surface, encoder: &mut CommandEncoder, screen: Option<&RenderTexture>) {
        self.uniforms.update(&surface.queue().borrow_mut());
        
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("VR Compositor render_pass"),
            color_attachments: &[screen.unwrap_or(&self.target).to_color_attachment()],
            depth_stencil_attachment: None,
        });
    
        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, &self.uniforms.bind_group, &[]);
        render_pass.set_bind_group(1, &self.source_bind_group, &[]);
        render_pass.draw(0..6, 0..1);
    }
}
