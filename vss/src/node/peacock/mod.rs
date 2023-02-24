use super::*;

struct Uniforms{
    cb_cpu: f32,
    cb_cpv: f32,
    cb_am: f32,
    cb_ayi: f32,

    track_error: i32,
    cb_monochrome: i32,
    cb_strength: f32,
}

pub struct PeacockCB {
    pipeline: wgpu::RenderPipeline,
    uniforms: ShaderUniforms<Uniforms>,
    sources_bind_group: wgpu::BindGroup,
    targets: ColorTargets,
}

impl PeacockCB{}

impl Node for PeacockCB {
    fn new(window: &Window) -> Self {
        let device = window.device().borrow_mut();
        let queue = window.queue().borrow_mut();

        let uniforms = ShaderUniforms::new(&device, 
            Uniforms{
                cb_cpu: 0.0,
                cb_cpv: 0.0,
                cb_am: 0.0,
                cb_ayi: 0.0,
                track_error: 0,
                cb_monochrome: 0,
                cb_strength: 0.0,
            });

        let (sources_bind_group_layout, sources_bind_group) = create_color_sources_bind_group(&device, &queue, "Peacock");

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Peacock Shader"),
            source: wgpu::ShaderSource::Wgsl(concat!(
                include_str!("../vert.wgsl"),
                include_str!("mod.wgsl")).into()),
        });

        let pipeline = create_render_pipeline(
            &device,
            &[&shader, &shader],
            &["vs_main", "fs_main"],
            &[&uniforms.bind_group_layout, &sources_bind_group_layout],
            &all_color_states(),
            None,
            Some("Peacock Render Pipeline"));

        PeacockCB {
            pipeline,
            uniforms,
            sources_bind_group,
            targets: ColorTargets::new(&device, "Peacock"),
        }
    }

    fn negociate_slots(&mut self, window: &Window, slots: NodeSlots) -> NodeSlots {
        let slots = slots.to_color_input(window).to_color_output(window, "PeacockNode");
        let device = window.device().borrow_mut();

        self.sources_bind_group = slots.as_all_colors_source(&device);
        self.targets = slots.as_all_colors_target();

        slots
    }

    fn update_values(&mut self, _window: &Window, values: &ValueMap) {
        let v_cpu: [f32; 3] = [0.753, 1.140, 0.171];
        let v_cpv: [f32; 3] = [0.265,-0.140,-0.003];
        let v_am: [f32; 3] = [1.273463, 0.968437, 0.062921];
        let v_ayi: [f32; 3] = [-0.073894, 0.003331, 0.292119];
        if let Some(Value::Bool(true)) = values.get("peacock_cb_onoff") {
            if let Some(Value::Number(cb_strength)) = values.get("peacock_cb_strength") {
                self.uniforms.data.cb_strength = *cb_strength as f32;
            }
            if let Some(Value::Number(cb_type)) = values.get("peacock_cb_type") {
                let cb_type = *cb_type as usize;
                if cb_type < 3 {
                    self.uniforms.data.cb_cpu = v_cpu[cb_type];
                    self.uniforms.data.cb_cpv = v_cpv[cb_type];
                    self.uniforms.data.cb_am = v_am[cb_type];
                    self.uniforms.data.cb_ayi = v_ayi[cb_type];
                    self.uniforms.data.cb_monochrome = 0;
                }else{
                    self.uniforms.data.cb_monochrome = 1;
                }
            }
        }else{
            self.uniforms.data.cb_strength = 0.0;
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
            color_attachments: &self.targets.color_attachments(screen),
            depth_stencil_attachment: None,
        });
    
        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, &self.uniforms.bind_group, &[]);
        render_pass.set_bind_group(1, &self.sources_bind_group, &[]);
        render_pass.draw(0..6, 0..1);
    }
}
