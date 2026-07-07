use super::*;

#[derive(Copy, Clone, PartialEq)]
struct Uniforms {
    resolution: [f32; 2],
    blur_factor: f32,
    contrast_factor: f32,
    active: i32,
    track_error: i32,
    _padding: [i32; 2],
}

pub struct Cataract {
    config: CataractConfig,
    color_pipeline: wgpu::RenderPipeline,
    metrics_ab_pipeline: wgpu::RenderPipeline,
    metrics_cd_pipeline: wgpu::RenderPipeline,
    uniforms: ShaderUniforms<Uniforms>,
    sources_bind_group: wgpu::BindGroup,
    targets: ColorDepthTargets,
    slots_active: bool,
}

pub struct CataractConfig {
    pub active: bool,
    pub blur_factor: f64,
    pub contrast_factor: f64,
    pub track_error: bool,
}

impl Default for CataractConfig {
    fn default() -> Self {
        Self {
            active: false,
            blur_factor: 0.0,
            contrast_factor: 0.0,
            track_error: false,
        }
    }
}

impl NodeConfig for CataractConfig {
    fn inspect(&mut self, inspector: &dyn Inspector) -> bool {
        let mut changed = false;
        changed |= inspector.mut_bool("ct_onoff", &mut self.active);
        changed |= inspector.mut_f64("ct_blur_factor", &mut self.blur_factor);
        changed |= inspector.mut_f64("ct_contrast_factor", &mut self.contrast_factor);
        changed |= inspector.mut_bool("track_error", &mut self.track_error);
        changed
    }
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
            &single_color_state(context.output_format()),
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
            config: CataractConfig::default(),
            color_pipeline,
            metrics_ab_pipeline,
            metrics_cd_pipeline,
            uniforms,
            sources_bind_group,
            targets: ColorDepthTargets::new(device, "Cataract"),
            slots_active: false,
        }
    }

    fn slots_active(&self) -> bool {
        self.config.active || self.config.track_error
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
        if !self.slots_active {
            return slots.to_passthrough();
        }

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

    fn inspect_config(&mut self, inspector: &dyn Inspector) -> bool {
        inspect_node_config(inspector, self.name(), &mut self.config)
    }

    fn configure(&mut self) -> NodeChanges {
        let slots_active = self.slots_active();
        let scale = if self.config.active { 0.01 } else { 0.0 };
        let uniforms = Uniforms {
            resolution: self.uniforms.data.resolution,
            blur_factor: self.config.blur_factor as f32 * scale,
            contrast_factor: self.config.contrast_factor as f32 * scale,
            active: self.config.active as i32,
            track_error: self.config.track_error as i32,
            _padding: [0, 0],
        };
        let output_changed = self.uniforms.data != uniforms;
        let slots_changed = self.slots_active != slots_active;

        self.slots_active = slots_active;
        self.uniforms.data = uniforms;

        NodeChanges::from_output_slots(output_changed, slots_changed)
    }

    fn render(
        &mut self,
        context: &RenderContext,
        encoder: &mut CommandEncoder,
        screen: Option<&RenderTexture>,
    ) {
        if !self.slots_active {
            return;
        }

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

        if self.config.track_error {
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

        if self.config.track_error {
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
