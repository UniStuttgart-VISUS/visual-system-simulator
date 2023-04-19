use super::*;

struct Uniforms {
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

    peacock_cb_onoff: bool,
    peacock_cb_type: i32,
}

impl PeacockCB {
    pub fn new(surface: &Surface) -> Self {
        let device = surface.device();
        let queue = surface.queue();

        let uniforms = ShaderUniforms::new(
            device,
            Uniforms {
                cb_cpu: 0.0,
                cb_cpv: 0.0,
                cb_am: 0.0,
                cb_ayi: 0.0,
                track_error: 0,
                cb_monochrome: 0,
                cb_strength: 0.0,
            },
        );

        let (sources_bind_group_layout, sources_bind_group) =
            create_color_sources_bind_group(device, queue, "Peacock");

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Peacock Shader"),
            source: wgpu::ShaderSource::Wgsl(
                concat!(include_str!("../vert.wgsl"), include_str!("mod.wgsl")).into(),
            ),
        });

        let pipeline = create_render_pipeline(
            device,
            &[&shader, &shader],
            &["vs_main", "fs_main"],
            &[&uniforms.bind_group_layout, &sources_bind_group_layout],
            &all_color_states(),
            None,
            Some("Peacock Render Pipeline"),
        );

        PeacockCB {
            pipeline,
            uniforms,
            sources_bind_group,
            targets: ColorTargets::new(device, "Peacock"),

            peacock_cb_onoff: false,
            peacock_cb_type: 0,
        }
    }
}

impl Node for PeacockCB {
    fn negociate_slots(
        &mut self,
        surface: &Surface,
        slots: NodeSlots,
        _original_image: &mut Option<Texture>,
    ) -> NodeSlots {
        let slots = slots
            .to_color_input(surface)
            .to_color_output(surface, "PeacockNode");
        let device = surface.device();

        self.sources_bind_group = slots.as_all_colors_source(device);
        self.targets = slots.as_all_colors_target();

        slots
    }

    fn inspect(&mut self, inspector: &mut dyn Inspector) {
        inspector.begin_node("Peacock");

        const V_CPU: [f32; 3] = [0.753, 1.140, 0.171];
        const V_CPV: [f32; 3] = [0.265, -0.140, -0.003];
        const V_AM: [f32; 3] = [1.273463, 0.968437, 0.062921];
        const V_AYI: [f32; 3] = [-0.073894, 0.003331, 0.292119];

        inspector.mut_bool("peacock_cb_onoff", &mut self.peacock_cb_onoff);

        inspector.mut_f32("peacock_cb_strength", &mut self.uniforms.data.cb_strength);

        inspector.mut_i32("peacock_cb_type", &mut self.peacock_cb_type);
        if self.peacock_cb_onoff {
            let cb_type = self.peacock_cb_type as usize;
            if cb_type < 3 {
                self.uniforms.data.cb_cpu = V_CPU[cb_type];
                self.uniforms.data.cb_cpv = V_CPV[cb_type];
                self.uniforms.data.cb_am = V_AM[cb_type];
                self.uniforms.data.cb_ayi = V_AYI[cb_type];
                self.uniforms.data.cb_monochrome = 0;
            } else {
                self.uniforms.data.cb_monochrome = 1;
            }
        } else {
            self.uniforms.data.cb_strength = 0.0;
        }

        inspector.end_node();
    }

    fn input(
        &mut self,
        perspective: &EyePerspective,
        vis_param: &VisualizationParameters,
    ) -> EyePerspective {
        self.uniforms.data.track_error = vis_param.has_to_track_error() as i32;
        perspective.clone()
    }

    fn render(
        &mut self,
        surface: &Surface,
        encoder: &mut CommandEncoder,
        screen: Option<&RenderTexture>,
    ) {
        self.uniforms.upload(surface.queue());

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
