mod retina_map;

use self::retina_map::*;
use super::*;
use cgmath::{Matrix4, Point3, SquareMatrix, Vector3};

struct Uniforms {
    proj: [[f32; 4]; 4],
    resolution: [f32; 2],
    achromatopsia_blur_factor: f32,
    track_error: i32,
}

pub struct Retina {
    pipeline: wgpu::RenderPipeline,
    uniforms: ShaderUniforms<Uniforms>,
    sources_bind_group: wgpu::BindGroup,
    retina_bind_group: wgpu::BindGroup,
    targets: ColorTargets,

    map_valid: bool,
    retina_map_pos_x_path: String,
    retina_map_neg_x_path: String,
    retina_map_pos_y_path: String,
    retina_map_neg_y_path: String,
    retina_map_pos_z_path: String,
    retina_map_neg_z_path: String,
    proj_matrix: Matrix4<f32>,
    cubemap_scale: f64,

    retina_map_builder: RetinaMapBuilder,
}

impl Retina {
    pub fn new(surface: &Surface) -> Self {
        let device = surface.device();
        let queue = surface.queue();

        let uniforms = ShaderUniforms::new(
            device,
            Uniforms {
                proj: [[0.0; 4]; 4],
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

        let pipeline = create_render_pipeline(
            device,
            &[&shader, &shader],
            &["vs_main", "fs_main"],
            &[
                &uniforms.bind_group_layout,
                &sources_bind_group_layout,
                &retina_layout,
            ],
            &all_color_states(),
            None,
            Some("Retina Render Pipeline"),
        );

        Retina {
            pipeline,
            uniforms,
            sources_bind_group,
            retina_bind_group,
            targets: ColorTargets::new(device, "Retina"),

            map_valid: false,
            retina_map_pos_x_path: String::new(),
            retina_map_neg_x_path: String::new(),
            retina_map_pos_y_path: String::new(),
            retina_map_neg_y_path: String::new(),
            retina_map_pos_z_path: String::new(),
            retina_map_neg_z_path: String::new(),
            proj_matrix: Matrix4::from_scale(1.0),
            cubemap_scale: 1.0,
            retina_map_builder: RetinaMapBuilder::new(),
        }
    }

    fn validate_map(&mut self, surface: &Surface) {
        if self.map_valid {
            return;
        }

        let device = surface.device();
        let queue = surface.queue();

        let mut image_data = Vec::new();
        if !self.retina_map_pos_x_path.is_empty() {
            image_data.push(load(&self.retina_map_pos_x_path));
        }
        if !self.retina_map_neg_x_path.is_empty() {
            image_data.push(load(&self.retina_map_neg_x_path));
        }
        if !self.retina_map_pos_y_path.is_empty() {
            image_data.push(load(&self.retina_map_pos_y_path));
        }
        if !self.retina_map_neg_y_path.is_empty() {
            image_data.push(load(&self.retina_map_neg_y_path));
        }
        if !self.retina_map_pos_z_path.is_empty() {
            image_data.push(load(&self.retina_map_pos_z_path));
        }
        if !self.retina_map_neg_z_path.is_empty() {
            image_data.push(load(&self.retina_map_neg_z_path));
        }

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
            let projection = self.proj_matrix;
            let res_x = self.uniforms.data.resolution[0] * 2.0 * projection[0][0];
            let res_y = self.uniforms.data.resolution[1] * 2.0 * projection[1][1];
            let mut resolution = res_x.max(res_y);
            if self.cubemap_scale > 0.0 {
                resolution *= self.cubemap_scale as f32;
            }
            let clamped_res = resolution.max(1.0) as u32;
            let cubemap_resolution = (clamped_res, clamped_res);

            //orientations directly taken from https://www.khronos.org/opengl/wiki/Cubemap_Texture
            let retina_map_pos_x = self.retina_map_builder.generate(
                cubemap_resolution,
                &[-Vector3::unit_z(), -Vector3::unit_y(), Vector3::unit_x()],
            );
            let retina_map_neg_x = self.retina_map_builder.generate(
                cubemap_resolution,
                &[Vector3::unit_z(), Vector3::unit_y(), -Vector3::unit_x()],
            );
            let retina_map_pos_y = self.retina_map_builder.generate(
                cubemap_resolution,
                &[Vector3::unit_x(), Vector3::unit_z(), Vector3::unit_y()],
            );
            let retina_map_neg_y = self.retina_map_builder.generate(
                cubemap_resolution,
                &[Vector3::unit_x(), -Vector3::unit_z(), -Vector3::unit_y()],
            );
            let retina_map_pos_z = self.retina_map_builder.generate(
                cubemap_resolution,
                &[Vector3::unit_x(), -Vector3::unit_y(), Vector3::unit_z()],
            );
            let retina_map_neg_z = self.retina_map_builder.generate(
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
    fn negociate_slots(
        &mut self,
        surface: &Surface,
        slots: NodeSlots,
        _original_image: &mut Option<Texture>,
    ) -> NodeSlots {
        let slots = slots
            .to_color_input(surface)
            .to_color_output(surface, "RetinaNode");
        self.uniforms.data.resolution = slots.output_size_f32();

        let device = surface.device();

        self.sources_bind_group = slots.as_all_colors_source(device);
        self.targets = slots.as_all_colors_target();
        slots
    }

    fn inspect(&mut self, inspector: &mut dyn Inspector) {
        inspector.begin_node("Retina");

        if inspector.mut_img("retina_map_pos_x_path", &mut self.retina_map_pos_x_path) {
            self.map_valid = false;
        }
        if inspector.mut_img("retina_map_neg_x_path", &mut self.retina_map_neg_x_path) {
            self.map_valid = false;
        }
        if inspector.mut_img("retina_map_pos_y_path", &mut self.retina_map_pos_y_path) {
            self.map_valid = false;
        }
        if inspector.mut_img("retina_map_neg_y_path", &mut self.retina_map_neg_y_path) {
            self.map_valid = false;
        }
        if inspector.mut_img("retina_map_pos_z_path", &mut self.retina_map_pos_z_path) {
            self.map_valid = false;
        }
        if inspector.mut_img("retina_map_neg_z_path", &mut self.retina_map_neg_z_path) {
            self.map_valid = false;
        }
        inspector.mut_f32(
            "achromatopsia_blur_factor",
            &mut self.uniforms.data.achromatopsia_blur_factor,
        );

        inspector.mut_matrix("proj_matrix", &mut self.proj_matrix);
        inspector.mut_f64("cubemap_scale", &mut self.cubemap_scale);

        self.retina_map_builder.inspect(inspector);

        inspector.end_node();
    }

    fn input(
        &mut self,
        perspective: &EyePerspective,
        vis_param: &VisualizationParameters,
    ) -> EyePerspective {
        let gaze_rotation = Matrix4::look_to_lh(
            Point3::new(0.0, 0.0, 0.0),
            perspective.gaze,
            Vector3::unit_y(),
        );
        //let gaze_rotation = Matrix4::from_scale(1.0);
        self.uniforms.data.proj = (gaze_rotation * perspective.proj.invert().unwrap()).into();
        //self.pso_data.u_proj = (head.proj * (Matrix4::from_translation(-head.position) * head.view)).into();
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
        self.validate_map(surface);

        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Retina render_pass"),
            color_attachments: &self.targets.color_attachments(screen),
            depth_stencil_attachment: None,
        });

        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, &self.uniforms.bind_group, &[]);
        render_pass.set_bind_group(1, &self.sources_bind_group, &[]);
        render_pass.set_bind_group(2, &self.retina_bind_group, &[]);
        render_pass.draw(0..6, 0..1);
    }
}
