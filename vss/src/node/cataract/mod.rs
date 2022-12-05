use super::*;

struct Uniforms{
    resolution: [f32; 2],
    blur_factor: f32,
    contrast_factor: f32,
    active: i32,
    track_error: i32,
}

struct Targets{
    rt_color: RenderTexture,
    rt_deflection: RenderTexture,
    rt_color_change: RenderTexture,
    rt_color_uncertainty: RenderTexture,
    rt_covariances: RenderTexture,
}

pub struct Cataract {
    pipeline: wgpu::RenderPipeline,
    uniforms: ShaderUniforms<Uniforms>,
    sources_bind_group: wgpu::BindGroup,
    targets: Targets,
}

impl Node for Cataract {
    fn new(window: &Window) -> Self {
        let device = window.device().borrow_mut();
        let queue = window.queue().borrow_mut();

        let uniforms = ShaderUniforms::new(&device, 
            Uniforms{
                resolution: [1.0, 1.0],
                blur_factor: 0.0,
                contrast_factor: 0.0,
                active: 0,
                track_error: 0,
            });

            let (sources_bind_group_layout, sources_bind_group) = create_textures_bind_group(
                &device,
                &[
                    &placeholder_texture(&device, &queue, Some("Cataract s_color")).unwrap(),
                    &placeholder_highp_texture(&device, &queue, Some("Cataract s_deflection")).unwrap(),
                    &placeholder_highp_texture(&device, &queue, Some("Cataract s_color_change")).unwrap(),
                    &placeholder_highp_texture(&device, &queue, Some("Cataract s_color_uncertainty")).unwrap(),
                    &placeholder_highp_texture(&device, &queue, Some("Cataract s_covariances")).unwrap(),
                ],
            );
    
            let rt_color = placeholder_color_rt(&device, Some("Cataract rt_color"));
            let rt_deflection = placeholder_highp_rt(&device, Some("Cataract rt_deflection"));
            let rt_color_change = placeholder_highp_rt(&device, Some("Cataract rt_color_change"));
            let rt_color_uncertainty = placeholder_highp_rt(&device, Some("Cataract rt_color_uncertainty"));
            let rt_covariances = placeholder_highp_rt(&device, Some("Cataract rt_covariances"));
            
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
                &[
                    blended_color_state(ColorFormat),
                    simple_color_state(HighpFormat),
                    simple_color_state(HighpFormat),
                    simple_color_state(HighpFormat),
                    simple_color_state(HighpFormat),
                    ],
                    None,
                Some("Peacock Render Pipeline"));

        Cataract {
            pipeline,
            uniforms,
            sources_bind_group: sources_bind_group,
            targets: Targets{
                rt_color,
                rt_deflection,
                rt_color_change,
                rt_color_uncertainty,
                rt_covariances,
            },
        }
    }

    fn negociate_slots(&mut self, window: &Window, slots: NodeSlots) -> NodeSlots {
        let slots = slots.to_color_input(window).to_color_output(window);
        self.uniforms.data.resolution = slots.output_size_f32();

        let slots = slots.to_color_input(window).to_color_output(window);
        let device = window.device().borrow_mut();

        self.sources_bind_group = slots.as_all_colors_source(&device);
        (self.targets.rt_color,
            self.targets.rt_deflection,
            self.targets.rt_color_change,
            self.targets.rt_color_uncertainty,
            self.targets.rt_covariances) = slots.as_all_color_output();

        slots
    }

    fn update_values(&mut self, _window: &Window, values: &ValueMap) {
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

    fn render(&mut self, window: &window::Window, encoder: &mut CommandEncoder, screen: Option<&RenderTexture>) {
        self.uniforms.update(&window.queue().borrow_mut());
        
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Peacock render_pass"),
            color_attachments: &[
                screen.unwrap_or(&self.targets.rt_color).to_color_attachment(),
                self.targets.rt_deflection.to_color_attachment(),
                self.targets.rt_color_change.to_color_attachment(),
                self.targets.rt_color_uncertainty.to_color_attachment(),
                self.targets.rt_covariances.to_color_attachment(),
                ],
            depth_stencil_attachment: None,
        });
    
        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, &self.uniforms.bind_group, &[]);
        render_pass.set_bind_group(1, &self.sources_bind_group, &[]);
        render_pass.draw(0..6, 0..1);
    }
}
