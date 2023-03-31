use super::*;

struct Uniforms{
    resolution: [f32; 2],
    blur_factor: f32,
    contrast_factor: f32,
    active: i32,
    track_error: i32,
}

pub struct Cataract {
    pipeline: wgpu::RenderPipeline,
    uniforms: ShaderUniforms<Uniforms>,
    sources_bind_group: wgpu::BindGroup,
    targets: ColorDepthTargets,
}

impl Cataract {
    pub fn new(surface: &Surface) -> Self {
        let device = surface.device().borrow_mut();
        let queue = surface.queue().borrow_mut();

        let uniforms = ShaderUniforms::new(&device, 
            Uniforms{
                resolution: [1.0, 1.0],
                blur_factor: 0.0,
                contrast_factor: 0.0,
                active: 0,
                track_error: 0,
            });

        let (sources_bind_group_layout, sources_bind_group) = create_color_depth_sources_bind_group(&device, &queue, "Cataract");

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Cataract Shader"),
            source: wgpu::ShaderSource::Wgsl(concat!(
                include_str!("../common.wgsl"),
                include_str!("../vert.wgsl"),
                include_str!("mod.wgsl")).into()),
        });

        let pipeline = create_render_pipeline(
            &device,
            &[&shader, &shader],
            &["vs_main", "fs_main"],
            &[&uniforms.bind_group_layout, &sources_bind_group_layout],
            &all_color_states(),
            simple_depth_state(DEPTH_FORMAT),
            Some("Cataract Render Pipeline"));

        Cataract {
            pipeline,
            uniforms,
            sources_bind_group,
            targets: ColorDepthTargets::new(&device, "Cataract"),
        }
    }
}

impl Node for Cataract {
    

    fn negociate_slots(&mut self, surface: &Surface, slots: NodeSlots, _original_image: &mut Option<Texture>) -> NodeSlots {
        let slots = slots.to_color_depth_input(surface).to_color_depth_output(surface, "CataractNode");
        self.uniforms.data.resolution = slots.output_size_f32();

        let device = surface.device().borrow_mut();

        self.sources_bind_group = slots.as_all_source(&device);
        self.targets = slots.as_all_target();

        slots
    }

    fn update_values(&mut self, _surface: &Surface, values: &ValueMap) {
        if let Some(Value::Bool(true)) = values.get("ct_onoff") {
            self.uniforms.data.active = 1;
            if let Some(Value::Number(ct_blur_factor)) = values.get("ct_blur_factor") {
                // ct_blur_factor is between 0 and 100
                self.uniforms.data.blur_factor = (*ct_blur_factor as f32) / 100.0;
            }
            if let Some(Value::Number(ct_contrast_factor)) = values.get("ct_contrast_factor") {
                //  ct_contrast_factor is between 0 and 100
                self.uniforms.data.contrast_factor = (*ct_contrast_factor as f32) / 100.0;
            }
        } else {
            self.uniforms.data.active = 0;
            self.uniforms.data.blur_factor = 0.0;
            self.uniforms.data.contrast_factor = 0.0;
        }
    }

    fn input(&mut self, perspective: &EyePerspective, vis_param: &VisualizationParameters) -> EyePerspective {        
        self.uniforms.data.track_error = vis_param.has_to_track_error() as i32;        
        perspective.clone()
    }

    fn render(&mut self, surface: &Surface, encoder: &mut CommandEncoder, screen: Option<&RenderTexture>) {
        self.uniforms.update(&surface.queue().borrow_mut());
        
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Cataract render_pass"),
            color_attachments: &self.targets.color_attachments(screen),
            depth_stencil_attachment: self.targets.depth_attachment(),
        });
    
        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, &self.uniforms.bind_group, &[]);
        render_pass.set_bind_group(1, &self.sources_bind_group, &[]);
        render_pass.draw(0..6, 0..1);
    }
}
