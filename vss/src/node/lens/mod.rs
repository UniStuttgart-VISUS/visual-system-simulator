mod generator;

pub use generator::*;

use super::*;

const DIOPTRES_SCALING: f64 = 0.332_763_369_417_523;

#[derive(Copy, Clone, PartialEq)]
struct Uniforms {
    lens_position: [f32; 2],

    active: i32,
    samplecount: i32,
    depth_min: f32,
    depth_max: f32,

    // smallest distance on which the eye can focus, in mm
    near_point: f32,

    // largest  distance on which the eye can focus, in mm
    far_point: f32,

    // determines the bluriness of objects that are too close to focus
    // should be between 0 and 2
    near_vision_factor: f32,

    // determines the bluriness of objects that are too far to focus
    // should be between 0 and 2
    far_vision_factor: f32,

    astigmatism_ecc_mm: f32,
    astigmatism_angle_deg: f32,
    eye_distance_center: f32,
    track_error: i32,
}

pub struct Lens {
    config: LensConfig,
    generator: NormalMapGenerator,
    color_pipeline: wgpu::RenderPipeline,
    metrics_ab_pipeline: wgpu::RenderPipeline,
    metrics_cd_pipeline: wgpu::RenderPipeline,
    uniforms: ShaderUniforms<Uniforms>,
    sources_bind_group: wgpu::BindGroup,
    normal_bind_group: wgpu::BindGroup,
    cornea_bind_group: wgpu::BindGroup,
    targets: ColorTargets,
    slots_active: bool,
}

pub struct LensConfig {
    samplecount: i32,
    presbyopia_onoff: bool,
    presbyopia_near_point: f64,
    myopiahyperopia_onoff: bool,
    myopiahyperopia_mnh: f64,
    astigmatism_dpt: f64,
    astigmatism_angle_deg: f32,
    eye_distance_center: f32,
    depth_min: f32,
    depth_max: f32,
    track_error: bool,
}

impl Default for LensConfig {
    fn default() -> Self {
        Self {
            samplecount: 4,
            presbyopia_onoff: false,
            presbyopia_near_point: 0.0,
            myopiahyperopia_onoff: false,
            myopiahyperopia_mnh: 0.0,
            astigmatism_dpt: 0.0,
            astigmatism_angle_deg: 0.0,
            eye_distance_center: 0.0,
            depth_min: 200.0,
            depth_max: 5000.0,
            track_error: false,
        }
    }
}

impl NodeConfig for LensConfig {
    fn inspect(&mut self, inspector: &dyn Inspector) -> bool {
        let mut changed = false;
        changed |= inspector.mut_i32("rays", &mut self.samplecount);
        changed |= inspector.mut_bool("presbyopia_onoff", &mut self.presbyopia_onoff);
        changed |= inspector.mut_f64("presbyopia_near_point", &mut self.presbyopia_near_point);
        changed |= inspector.mut_bool("myopiahyperopia_onoff", &mut self.myopiahyperopia_onoff);
        changed |= inspector.mut_f64("myopiahyperopia_mnh", &mut self.myopiahyperopia_mnh);
        changed |= inspector.mut_f64("astigmatism_dpt", &mut self.astigmatism_dpt);
        changed |= inspector.mut_f32("astigmatism_angle_deg", &mut self.astigmatism_angle_deg);
        changed |= inspector.mut_f32("eye_distance_center", &mut self.eye_distance_center);
        changed |= inspector.mut_f32("depth_min", &mut self.depth_min);
        changed |= inspector.mut_f32("depth_max", &mut self.depth_max);
        changed |= inspector.mut_bool("track_error", &mut self.track_error);
        changed
    }
}

impl Lens {
    pub fn new(context: &RenderContext) -> Self {
        let generator = NormalMapGenerator::new(context);
        let device = context.device();
        let queue = context.queue();

        let uniforms = ShaderUniforms::new(
            device,
            Uniforms {
                lens_position: [0.0, 0.0],
                active: 0,
                samplecount: 4,
                depth_min: 200.0,  //XXX: was 1000.0 - 300.0,
                depth_max: 5000.0, //XXX: was 1000.0 + 0.0,
                near_point: 0.0,
                far_point: f32::INFINITY,
                near_vision_factor: 0.0,
                far_vision_factor: 0.0,
                astigmatism_ecc_mm: 0.0,
                astigmatism_angle_deg: 0.0,
                eye_distance_center: 0.0,
                track_error: 0,
            },
        );

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Lens Shader"),
            source: wgpu::ShaderSource::Wgsl(
                concat!(
                    include_str!("../common.wgsl"),
                    include_str!("../vert.wgsl"),
                    include_str!("lens_model.wgsl"),
                    include_str!("mod.wgsl")
                )
                .into(),
            ),
        });

        let (normal_layout, normal_bind_group) =
            placeholder_highp_texture(device, queue, Some("Lens-Normal Texture placeholder"))
                .unwrap()
                .create_bind_group(device);

        let (cornea_layout, cornea_bind_group) = load_texture_from_bytes(
            device,
            queue,
            &[127, 127, 0, 0],
            1,
            1,
            create_sampler_linear(device),
            wgpu::TextureFormat::Rgba8Unorm,
            Some("Lens-Cornea Texture placeholder"),
        )
        .unwrap()
        .create_bind_group(device);

        let (sources_bind_group_layout, sources_bind_group) =
            create_color_depth_sources_bind_group(device, queue, "Cataract");

        let color_pipeline = create_render_pipeline(
            device,
            &[&shader, &shader],
            &["vs_main", "fs_color"],
            &[
                &uniforms.bind_group_layout,
                &sources_bind_group_layout,
                &normal_layout,
                &cornea_layout,
            ],
            &single_color_state(context.output_format()),
            None,
            Some("Lens Color Render Pipeline"),
        );

        let metrics_ab_pipeline = create_render_pipeline(
            device,
            &[&shader, &shader],
            &["vs_main", "fs_metrics_ab"],
            &[
                &uniforms.bind_group_layout,
                &sources_bind_group_layout,
                &normal_layout,
                &cornea_layout,
            ],
            &metrics_ab_color_states(),
            None,
            Some("Lens Metrics AB Render Pipeline"),
        );

        let metrics_cd_pipeline = create_render_pipeline(
            device,
            &[&shader, &shader],
            &["vs_main", "fs_metrics_cd"],
            &[
                &uniforms.bind_group_layout,
                &sources_bind_group_layout,
                &normal_layout,
                &cornea_layout,
            ],
            &metrics_cd_color_states(),
            None,
            Some("Lens Metrics CD Render Pipeline"),
        );

        Lens {
            config: LensConfig::default(),
            generator,
            color_pipeline,
            metrics_ab_pipeline,
            metrics_cd_pipeline,
            uniforms,
            sources_bind_group,
            normal_bind_group,
            cornea_bind_group,
            targets: ColorTargets::new(device, "Lens"),
            slots_active: false,
        }
    }

    fn slots_active(&self) -> bool {
        self.config.presbyopia_onoff || self.config.myopiahyperopia_onoff || self.config.track_error
    }
}

impl Node for Lens {
    fn name(&self) -> &'static str {
        "Lens"
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
            .to_color_metrics_output(context, "LensNode");
        let device = context.device();
        let queue = context.queue();

        self.sources_bind_group = slots.as_all_source(device, queue);
        self.targets = slots.as_all_colors_target();

        let size = slots.output_size_f32();
        self.generator
            .generate(device, queue, size[0] as u32, size[1] as u32);
        (_, self.normal_bind_group) = self.generator.texture.create_bind_group(device);

        slots
    }

    fn inspect_config(&mut self, inspector: &dyn Inspector) -> bool {
        inspect_node_config(inspector, self.name(), &mut self.config)
    }

    fn configure(&mut self) -> NodeChanges {
        let slots_active = self.slots_active();
        let mut active = 0;
        let mut near_point: f32 = 0.0;
        let mut far_point = f32::INFINITY;
        let mut near_vision_factor: f32 = 0.0;
        let mut far_vision_factor: f32 = 0.0;

        if self.config.presbyopia_onoff {
            active = 1;
            near_point = self.config.presbyopia_near_point as f32;
            near_vision_factor = 1.0;
        }

        if self.config.myopiahyperopia_onoff {
            active = 1;
            let dioptres = ((self.config.myopiahyperopia_mnh / 50.0 - 1.0) * 3.0) as f32;
            if dioptres < 0.0 {
                far_point = -1000.0 / dioptres;
                near_point = near_point.min(far_point);
                far_vision_factor = far_vision_factor.max(1.0 - dioptres * DIOPTRES_SCALING as f32);
            } else if dioptres > 0.0 {
                near_point = near_point.max(1000.0 / (4.4 - dioptres));
                near_vision_factor =
                    near_vision_factor.max(1.0 + dioptres * DIOPTRES_SCALING as f32);
            }
        }

        let astigmatism_ecc_mm = (0.2 * self.config.astigmatism_dpt) as f32;
        let uniforms = Uniforms {
            lens_position: self.uniforms.data.lens_position,
            active,
            samplecount: self.config.samplecount,
            depth_min: self.config.depth_min,
            depth_max: self.config.depth_max,
            near_point,
            far_point,
            near_vision_factor,
            far_vision_factor,
            astigmatism_ecc_mm,
            astigmatism_angle_deg: self.config.astigmatism_angle_deg,
            eye_distance_center: self.config.eye_distance_center,
            track_error: self.config.track_error as i32,
        };
        let output_changed = self.uniforms.data != uniforms;
        let slots_changed = self.slots_active != slots_active;

        self.slots_active = slots_active;
        self.uniforms.data = uniforms;

        NodeChanges::from_output_slots(output_changed, slots_changed)
    }

    fn input(&mut self, eye: &EyeInput, _mouse: &MouseInput) -> (EyeInput, NodeChanges) {
        let lens_position = [eye.position.x, eye.position.y];
        let output_changed = self.slots_active && self.uniforms.data.lens_position != lens_position;
        self.uniforms.data.lens_position = lens_position;
        (
            eye.clone(),
            NodeChanges::from_output_slots(output_changed, false),
        )
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
                label: Some("Lens color_pass"),
                color_attachments: &self.targets.color_attachments(screen),
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });

            render_pass.set_pipeline(&self.color_pipeline);
            render_pass.set_bind_group(0, &self.uniforms.bind_group, &[]);
            render_pass.set_bind_group(1, &self.sources_bind_group, &[]);
            render_pass.set_bind_group(2, &self.normal_bind_group, &[]);
            render_pass.set_bind_group(3, &self.cornea_bind_group, &[]);
            render_pass.draw(0..6, 0..1);
        }

        if self.config.track_error {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Lens metrics_ab_pass"),
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
            render_pass.set_bind_group(2, &self.normal_bind_group, &[]);
            render_pass.set_bind_group(3, &self.cornea_bind_group, &[]);
            render_pass.draw(0..6, 0..1);
        }

        if self.config.track_error {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Lens metrics_cd_pass"),
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
            render_pass.set_bind_group(2, &self.normal_bind_group, &[]);
            render_pass.set_bind_group(3, &self.cornea_bind_group, &[]);
            render_pass.draw(0..6, 0..1);
        }
    }
}
