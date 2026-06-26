use super::*;

struct Uniforms {
    resolution: [f32; 2],
    blur_factor: f32,
    contrast_factor: f32,
    active: i32,
    track_error: i32,
    _padding: [i32; 2],
}

pub struct Cataract {
    color_pipeline: wgpu::RenderPipeline,
    metrics_ab_pipeline: wgpu::RenderPipeline,
    metrics_cd_pipeline: wgpu::RenderPipeline,
    uniforms: ShaderUniforms<Uniforms>,
    sources_bind_group: wgpu::BindGroup,
    targets: ColorDepthTargets,

    track_error: bool,
}

impl Cataract {
    pub fn new(context: &RenderContext) -> Self {
        let device = context.device();
        let queue = context.queue();

        let uniforms = ShaderUniforms::new(
            device,
            Uniforms {
                resolution: [1.0, 1.0],
                blur_factor: 0.0,
                contrast_factor: 0.0,
                active: 0,
                track_error: 0,
                _padding: [0, 0],
            },
        );

        let (sources_bind_group_layout, sources_bind_group) =
            create_color_depth_sources_bind_group(device, queue, "Cataract");

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Cataract Shader"),
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
            simple_depth_state(DEPTH_FORMAT),
            Some("Cataract Color Render Pipeline"),
        );

        let metrics_ab_pipeline = create_render_pipeline(
            device,
            &[&shader, &shader],
            &["vs_main", "fs_metrics_ab"],
            &[&uniforms.bind_group_layout, &sources_bind_group_layout],
            &metrics_ab_color_states(),
            None,
            Some("Cataract Metrics AB Render Pipeline"),
        );

        let metrics_cd_pipeline = create_render_pipeline(
            device,
            &[&shader, &shader],
            &["vs_main", "fs_metrics_cd"],
            &[&uniforms.bind_group_layout, &sources_bind_group_layout],
            &metrics_cd_color_states(),
            None,
            Some("Cataract Metrics CD Render Pipeline"),
        );

        Cataract {
            color_pipeline,
            metrics_ab_pipeline,
            metrics_cd_pipeline,
            uniforms,
            sources_bind_group,
            targets: ColorDepthTargets::new(device, "Cataract"),
            track_error: false,
        }
    }
}

impl Node for Cataract {
    fn name(&self) -> &'static str {
        "Cataract"
    }

    fn negociate_slots(
        &mut self,
        context: &RenderContext,
        slots: NodeSlots,
        _original_image: &mut Option<Texture>,
    ) -> NodeSlots {
        let slots = slots
            .to_color_depth_metrics_input(context)
            .to_color_depth_metrics_output(context, "CataractNode");
        self.uniforms.data.resolution = slots.output_size_f32();

        let device = context.device();
        let queue = context.queue();

        self.sources_bind_group = slots.as_all_source(device, queue);
        self.targets = slots.as_all_target();

        slots
    }

    fn inspect(&mut self, inspector: &dyn Inspector) {
        let mut active = self.uniforms.data.active == 0;
        if inspector.mut_bool("ct_onoff", &mut active) {
            self.uniforms.data.active = active as i32;
        }
        if self.uniforms.data.active == 0 {
            self.uniforms.data.blur_factor = 0.0;
            self.uniforms.data.contrast_factor = 0.0;
        }

        // ct_blur_factor is between 0 and 100
        let mut blur_factor = (self.uniforms.data.blur_factor * 100.0) as f64;
        if inspector.mut_f64("ct_blur_factor", &mut blur_factor) {
            self.uniforms.data.blur_factor = (blur_factor as f32) / 100.0;
        }

        // ct_contrast_factor is between 0 and 100
        let mut contrast_factor = (self.uniforms.data.contrast_factor * 100.0) as f64;
        if inspector.mut_f64("ct_contrast_factor", &mut contrast_factor) {
            self.uniforms.data.contrast_factor = (contrast_factor as f32) / 100.0;
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
                label: Some("Cataract color_pass"),
                color_attachments: &self.targets.color_attachments(screen),
                depth_stencil_attachment: self.targets.depth_attachment(),
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
                label: Some("Cataract metrics_ab_pass"),
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
                label: Some("Cataract metrics_cd_pass"),
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
