use super::*;

struct Uniforms {
    cb_cpu: f32,
    cb_cpv: f32,
    cb_am: f32,
    cb_ayi: f32,

    track_error: i32,
    cb_monochrome: i32,
    cb_strength: f32,

    _padding: f32,
}

pub struct PeacockCB {
    color_pipeline: wgpu::RenderPipeline,
    metrics_ab_pipeline: wgpu::RenderPipeline,
    metrics_cd_pipeline: wgpu::RenderPipeline,
    uniforms: ShaderUniforms<Uniforms>,
    sources_bind_group: wgpu::BindGroup,
    targets: ColorTargets,

    peacock_cb_onoff: bool,
    peacock_cb_type: i32,
    track_error: bool,
}

impl PeacockCB {
    pub fn new(context: &RenderContext) -> Self {
        let device = context.device();
        let queue = context.queue();

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
                _padding: 0.0,
            },
        );

        let (sources_bind_group_layout, sources_bind_group) =
            create_color_sources_bind_group(device, queue, "Peacock");

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Peacock Shader"),
            source: wgpu::ShaderSource::Wgsl(
                concat!(
                    include_str!("../common.wgsl"),
                    include_str!("../vert.wgsl"),
                    include_str!("mod.wgsl")
                )
                .into(),
            ),
        });

        let color_pipeline = create_render_pipeline(
            device,
            &[&shader, &shader],
            &["vs_main", "fs_color"],
            &[&uniforms.bind_group_layout, &sources_bind_group_layout],
            &single_color_state(),
            None,
            Some("Peacock Color Render Pipeline"),
        );

        let metrics_ab_pipeline = create_render_pipeline(
            device,
            &[&shader, &shader],
            &["vs_main", "fs_metrics_ab"],
            &[&uniforms.bind_group_layout, &sources_bind_group_layout],
            &metrics_ab_color_states(),
            None,
            Some("Peacock Metrics AB Render Pipeline"),
        );

        let metrics_cd_pipeline = create_render_pipeline(
            device,
            &[&shader, &shader],
            &["vs_main", "fs_metrics_cd"],
            &[&uniforms.bind_group_layout, &sources_bind_group_layout],
            &metrics_cd_color_states(),
            None,
            Some("Peacock Metrics CD Render Pipeline"),
        );

        PeacockCB {
            color_pipeline,
            metrics_ab_pipeline,
            metrics_cd_pipeline,
            uniforms,
            sources_bind_group,
            targets: ColorTargets::new(device, "Peacock"),

            peacock_cb_onoff: false,
            peacock_cb_type: 0,

            track_error: false,
        }
    }
}

impl Node for PeacockCB {
    fn name(&self) -> &'static str {
        "PeacockCB"
    }

    fn negociate_slots(
        &mut self,
        context: &RenderContext,
        slots: NodeSlots,
        _original_image: &mut Option<Texture>,
    ) -> NodeSlots {
        let slots = slots
            .to_color_metrics_input(context)
            .to_color_metrics_output(context, "PeacockNode");
        let device = context.device();
        let queue = context.queue();

        self.sources_bind_group = slots.as_all_colors_source(device, queue);
        self.targets = slots.as_all_colors_target();

        slots
    }

    fn inspect(&mut self, inspector: &dyn Inspector) {
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

        inspector.mut_bool("track_error", &mut self.track_error);
    }

    fn render(
        &mut self,
        context: &RenderContext,
        encoder: &mut CommandEncoder,
        screen: Option<&RenderTexture>,
    ) {
        self.uniforms.data.track_error = self.track_error as i32;
        self.uniforms.upload(context.queue());

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Peacock color_pass"),
                color_attachments: &self.targets.color_attachments(screen),
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });

            render_pass.set_pipeline(&self.color_pipeline);
            render_pass.set_bind_group(0, &self.uniforms.bind_group, &[]);
            render_pass.set_bind_group(1, &self.sources_bind_group, &[]);
            render_pass.draw(0..6, 0..1);
        }

        if self.track_error {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Peacock metrics_ab_pass"),
                color_attachments: &[
                    self.targets
                        .rt_deflection
                        .to_color_attachment(Some(CLEAR_COLOR)),
                    self.targets
                        .rt_color_change
                        .to_color_attachment(Some(CLEAR_COLOR)),
                ],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });

            render_pass.set_pipeline(&self.metrics_ab_pipeline);
            render_pass.set_bind_group(0, &self.uniforms.bind_group, &[]);
            render_pass.set_bind_group(1, &self.sources_bind_group, &[]);
            render_pass.draw(0..6, 0..1);
        }

        if self.track_error {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Peacock metrics_cd_pass"),
                color_attachments: &[
                    self.targets
                        .rt_color_uncertainty
                        .to_color_attachment(Some(CLEAR_COLOR)),
                    self.targets
                        .rt_covariances
                        .to_color_attachment(Some(CLEAR_COLOR)),
                ],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });

            render_pass.set_pipeline(&self.metrics_cd_pipeline);
            render_pass.set_bind_group(0, &self.uniforms.bind_group, &[]);
            render_pass.set_bind_group(1, &self.sources_bind_group, &[]);
            render_pass.draw(0..6, 0..1);
        }
    }
}
