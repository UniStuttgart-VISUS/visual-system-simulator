mod retina_map;

use self::retina_map::*;
use super::*;
use cgmath::{Matrix4, Point3, SquareMatrix, Vector3};

#[repr(C)]
struct Uniforms {
    gaze_inv_proj: [[f32; 4]; 4],
    resolution: [f32; 2],
    achromatopsia_blur_factor: f32,
    track_error: i32,
}

#[derive(Clone, PartialEq)]
struct MapConfig {
    retina_map_pos_x_path: AssetId,
    retina_map_neg_x_path: AssetId,
    retina_map_pos_y_path: AssetId,
    retina_map_neg_y_path: AssetId,
    retina_map_pos_z_path: AssetId,
    retina_map_neg_z_path: AssetId,
    proj_matrix: Matrix4<f32>,
    cubemap_scale: f64,
    retina_map_builder: RetinaMapBuilder,
}

pub struct Retina {
    config: RetinaConfig,
    color_pipeline: wgpu::RenderPipeline,
    metrics_ab_pipeline: wgpu::RenderPipeline,
    metrics_cd_pipeline: wgpu::RenderPipeline,
    uniforms: ShaderUniforms<Uniforms>,
    sources_bind_group: wgpu::BindGroup,
    retina_bind_group: wgpu::BindGroup,
    targets: ColorTargets,

    map_valid: bool,
    map_config_changed: bool,
    track_error: bool,
}

pub struct RetinaConfig {
    retina_map_pos_x_path: AssetId,
    retina_map_neg_x_path: AssetId,
    retina_map_pos_y_path: AssetId,
    retina_map_neg_y_path: AssetId,
    retina_map_pos_z_path: AssetId,
    retina_map_neg_z_path: AssetId,
    achromatopsia_blur_factor: f32,
    proj_matrix: Matrix4<f32>,
    cubemap_scale: f64,
    track_error: bool,
    retina_map_builder: RetinaMapBuilder,
}

impl Default for RetinaConfig {
    fn default() -> Self {
        Self {
            retina_map_pos_x_path: AssetId::new(),
            retina_map_neg_x_path: AssetId::new(),
            retina_map_pos_y_path: AssetId::new(),
            retina_map_neg_y_path: AssetId::new(),
            retina_map_pos_z_path: AssetId::new(),
            retina_map_neg_z_path: AssetId::new(),
            achromatopsia_blur_factor: 0.0,
            proj_matrix: Matrix4::from_scale(1.0),
            cubemap_scale: 1.0,
            track_error: false,
            retina_map_builder: RetinaMapBuilder::new(),
        }
    }
}

impl NodeConfig for RetinaConfig {
    fn inspect(&mut self, inspector: &dyn Inspector) -> bool {
        let mut changed = false;
        changed |= inspector.mut_asset("retina_map_pos_x_path", &mut self.retina_map_pos_x_path);
        changed |= inspector.mut_asset("retina_map_neg_x_path", &mut self.retina_map_neg_x_path);
        changed |= inspector.mut_asset("retina_map_pos_y_path", &mut self.retina_map_pos_y_path);
        changed |= inspector.mut_asset("retina_map_neg_y_path", &mut self.retina_map_neg_y_path);
        changed |= inspector.mut_asset("retina_map_pos_z_path", &mut self.retina_map_pos_z_path);
        changed |= inspector.mut_asset("retina_map_neg_z_path", &mut self.retina_map_neg_z_path);
        changed |= inspector.mut_f32(
            "achromatopsia_blur_factor",
            &mut self.achromatopsia_blur_factor,
        );
        changed |= inspector.mut_matrix("proj_matrix", &mut self.proj_matrix);
        changed |= inspector.mut_f64("cubemap_scale", &mut self.cubemap_scale);
        changed |= inspector.mut_bool("track_error", &mut self.track_error);
        changed |= self.retina_map_builder.inspect(inspector);
        changed
    }
}

impl RetinaConfig {
    fn map_config(&self) -> MapConfig {
        MapConfig {
            retina_map_pos_x_path: self.retina_map_pos_x_path.clone(),
            retina_map_neg_x_path: self.retina_map_neg_x_path.clone(),
            retina_map_pos_y_path: self.retina_map_pos_y_path.clone(),
            retina_map_neg_y_path: self.retina_map_neg_y_path.clone(),
            retina_map_pos_z_path: self.retina_map_pos_z_path.clone(),
            retina_map_neg_z_path: self.retina_map_neg_z_path.clone(),
            proj_matrix: self.proj_matrix,
            cubemap_scale: self.cubemap_scale,
            retina_map_builder: self.retina_map_builder.clone(),
        }
    }
}

impl Retina {
    pub fn new(context: &RenderContext) -> Self {
        let device = context.device();
        let queue = context.queue();

        let uniforms = ShaderUniforms::new(
            device,
            Uniforms {
                gaze_inv_proj: [[0.0; 4]; 4],
                resolution: [0.0; 2],
                achromatopsia_blur_factor: 0.0,
                track_error: 0,
            },
        );

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Retina Shader"),
            source: wgpu::ShaderSource::Wgsl(
                concat!(
                    include_str!("../common.wgsl"),
                    include_str!("../vert.wgsl"),
                    include_str!("mod.wgsl")
                )
                .into(),
            ),
        });

        let (retina_layout, retina_bind_group) = load_cubemap_from_bytes(
            device,
            queue,
            &[0; 4 * 6],
            1,
            create_sampler_linear(device),
            wgpu::TextureFormat::Rgba8Unorm,
            Some("Retina Texture placeholder"),
        )
        .unwrap()
        .create_bind_group(device);

        let (sources_bind_group_layout, sources_bind_group) =
            create_color_sources_bind_group(device, queue, "Cataract");

        let color_pipeline = create_render_pipeline(
            device,
            &[&shader, &shader],
            &["vs_main", "fs_color"],
            &[
                &uniforms.bind_group_layout,
                &sources_bind_group_layout,
                &retina_layout,
            ],
            &single_color_state(context.output_format()),
            None,
            Some("Retina Color Render Pipeline"),
        );

        let metrics_ab_pipeline = create_render_pipeline(
            device,
            &[&shader, &shader],
            &["vs_main", "fs_metrics_ab"],
            &[
                &uniforms.bind_group_layout,
                &sources_bind_group_layout,
                &retina_layout,
            ],
            &metrics_ab_color_states(),
            None,
            Some("Retina Metrics AB Render Pipeline"),
        );

        let metrics_cd_pipeline = create_render_pipeline(
            device,
            &[&shader, &shader],
            &["vs_main", "fs_metrics_cd"],
            &[
                &uniforms.bind_group_layout,
                &sources_bind_group_layout,
                &retina_layout,
            ],
            &metrics_cd_color_states(),
            None,
            Some("Retina Metrics CD Render Pipeline"),
        );

        Retina {
            config: RetinaConfig::default(),
            color_pipeline,
            metrics_ab_pipeline,
            metrics_cd_pipeline,
            uniforms,
            sources_bind_group,
            retina_bind_group,
            targets: ColorTargets::new(device, "Retina"),

            map_valid: false,
            map_config_changed: false,
            track_error: false,
        }
    }

    fn validate_map(&mut self, context: &RenderContext) {
        if self.map_valid {
            return;
        }

        let device = context.device();
        let queue = context.queue();

        let mut image_data = Vec::new();
        let mut load_map = |path: &AssetId| {
            if path.is_empty() {
                return;
            }
            match load(path.raw()) {
                Ok(data) => image_data.push(data),
                Err(err) => panic!("failed to load retina map {}: {err}", path),
            }
        };
        load_map(&self.config.retina_map_pos_x_path);
        load_map(&self.config.retina_map_neg_x_path);
        load_map(&self.config.retina_map_pos_y_path);
        load_map(&self.config.retina_map_neg_y_path);
        load_map(&self.config.retina_map_pos_z_path);
        load_map(&self.config.retina_map_neg_z_path);

        if image_data.len() == 6 {
            (_, self.retina_bind_group) = load_cubemap(
                device,
                queue,
                image_data,
                create_sampler_linear(device),
                wgpu::TextureFormat::Rgba8Unorm,
                Some("Retina Texture from Images"),
            )
            .unwrap()
            .create_bind_group(device);
        } else {
            let projection = self.config.proj_matrix;
            let res_x = self.uniforms.data.resolution[0] * 2.0 * projection[0][0];
            let res_y = self.uniforms.data.resolution[1] * 2.0 * projection[1][1];
            let mut resolution = res_x.max(res_y);
            if self.config.cubemap_scale > 0.0 {
                resolution *= self.config.cubemap_scale as f32;
            }
            let clamped_res = resolution.max(1.0) as u32;
            let cubemap_resolution = (clamped_res, clamped_res);

            //orientations directly taken from https://www.khronos.org/opengl/wiki/Cubemap_Texture
            let retina_map_pos_x = self.config.retina_map_builder.generate(
                cubemap_resolution,
                &[-Vector3::unit_z(), -Vector3::unit_y(), Vector3::unit_x()],
            );
            let retina_map_neg_x = self.config.retina_map_builder.generate(
                cubemap_resolution,
                &[Vector3::unit_z(), Vector3::unit_y(), -Vector3::unit_x()],
            );
            let retina_map_pos_y = self.config.retina_map_builder.generate(
                cubemap_resolution,
                &[Vector3::unit_x(), Vector3::unit_z(), Vector3::unit_y()],
            );
            let retina_map_neg_y = self.config.retina_map_builder.generate(
                cubemap_resolution,
                &[Vector3::unit_x(), -Vector3::unit_z(), -Vector3::unit_y()],
            );
            let retina_map_pos_z = self.config.retina_map_builder.generate(
                cubemap_resolution,
                &[Vector3::unit_x(), -Vector3::unit_y(), Vector3::unit_z()],
            );
            let retina_map_neg_z = self.config.retina_map_builder.generate(
                cubemap_resolution,
                &[-Vector3::unit_x(), -Vector3::unit_y(), -Vector3::unit_z()],
            );
            //save latest retina map
            //let _ = image::save_buffer(&Path::new("last.retina_pos_x.png"), &retina_map_pos_x, cubemap_resolution.0, cubemap_resolution.1, image::ColorType::Rgba8);
            //let _ = image::save_buffer(&Path::new("last.retina_neg_x.png"), &retina_map_neg_x, cubemap_resolution.0, cubemap_resolution.1, image::ColorType::Rgba8);
            //let _ = image::save_buffer(&Path::new("last.retina_pos_y.png"), &retina_map_pos_y, cubemap_resolution.0, cubemap_resolution.1, image::ColorType::Rgba8);
            //let _ = image::save_buffer(&Path::new("last.retina_neg_y.png"), &retina_map_neg_y, cubemap_resolution.0, cubemap_resolution.1, image::ColorType::Rgba8);
            //let _ = image::save_buffer(&Path::new("last.retina_pos_z.png"), &retina_map_pos_z, cubemap_resolution.0, cubemap_resolution.1, image::ColorType::Rgba8);
            //let _ = image::save_buffer(&Path::new("last.retina_neg_z.png"), &retina_map_neg_z, cubemap_resolution.0, cubemap_resolution.1, image::ColorType::Rgba8);
            (_, self.retina_bind_group) = load_cubemap_from_bytes(
                device,
                queue,
                &([
                    retina_map_pos_x,
                    retina_map_neg_x,
                    retina_map_pos_y,
                    retina_map_neg_y,
                    retina_map_pos_z,
                    retina_map_neg_z,
                ]
                .concat()),
                cubemap_resolution.0,
                create_sampler_linear(device),
                wgpu::TextureFormat::Rgba8Unorm,
                Some("Retina Texture from bytes"),
            )
            .unwrap()
            .create_bind_group(device);
        };

        self.map_valid = true;
    }
}

impl Node for Retina {
    fn name(&self) -> &'static str {
        "Retina"
    }

    fn negociate_slots(
        &mut self,
        context: &RenderContext,
        slots: NodeSlots,
        _original_image: &mut Option<Texture>,
    ) -> NodeSlots {
        let slots = slots
            .to_color_metrics_input(context)
            .to_color_metrics_output(context, "RetinaNode");
        self.uniforms.data.resolution = slots.output_size_f32();

        let device = context.device();
        let queue = context.queue();

        self.sources_bind_group = slots.as_all_colors_source(device, queue);
        self.targets = slots.as_all_colors_target();
        slots
    }

    fn inspect_config(&mut self, inspector: &dyn Inspector) -> bool {
        let old_map_config = self.config.map_config();
        let changed = inspect_node_config(inspector, self.name(), &mut self.config);
        self.map_config_changed |= old_map_config != self.config.map_config();
        changed
    }

    fn configure(&mut self) -> NodeChanges {
        let track_error = self.config.track_error as i32;
        let output_changed = self.map_config_changed
            || self.uniforms.data.achromatopsia_blur_factor
                != self.config.achromatopsia_blur_factor
            || self.uniforms.data.track_error != track_error;

        if self.map_config_changed {
            self.map_valid = false;
            self.map_config_changed = false;
        }
        self.track_error = self.config.track_error;
        self.uniforms.data.achromatopsia_blur_factor = self.config.achromatopsia_blur_factor;
        self.uniforms.data.track_error = track_error;

        NodeChanges::from_output_slots(output_changed, false)
    }

    fn input(&mut self, eye: &EyeInput, _mouse: &MouseInput) -> (EyeInput, NodeChanges) {
        let gaze_rotation =
            Matrix4::look_to_lh(Point3::new(0.0, 0.0, 0.0), eye.gaze, Vector3::unit_y());
        let gaze_inv_proj = (gaze_rotation.invert().unwrap() * eye.proj.invert().unwrap()).into();
        let output_changed = self.uniforms.data.gaze_inv_proj != gaze_inv_proj;
        self.uniforms.data.gaze_inv_proj = gaze_inv_proj;

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
        self.uniforms.upload(context.queue());
        self.validate_map(context);

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Retina color_pass"),
                color_attachments: &self.targets.color_attachments(screen),
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });

            render_pass.set_pipeline(&self.color_pipeline);
            render_pass.set_bind_group(0, &self.uniforms.bind_group, &[]);
            render_pass.set_bind_group(1, &self.sources_bind_group, &[]);
            render_pass.set_bind_group(2, &self.retina_bind_group, &[]);
            render_pass.draw(0..6, 0..1);
        }

        if self.track_error {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Retina metrics_ab_pass"),
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
            render_pass.set_bind_group(2, &self.retina_bind_group, &[]);
            render_pass.draw(0..6, 0..1);
        }

        if self.track_error {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Retina metrics_cd_pass"),
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
            render_pass.set_bind_group(2, &self.retina_bind_group, &[]);
            render_pass.draw(0..6, 0..1);
        }
    }
}
