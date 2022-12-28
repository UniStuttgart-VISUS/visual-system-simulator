mod generator;

pub use generator::*;

use super::*;

const DIOPTRES_SCALING: f32 = 0.332_763_369_417_523 as f32;

struct Uniforms{
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

    dir_calc_scale: f32,
    astigmatism_ecc_mm: f32,
    astigmatism_angle_deg: f32,
    eye_distance_center: f32,
    track_error: i32,

    _padding: i32,
}

pub struct Lens {
    generator: NormalMapGenerator,
    pipeline: wgpu::RenderPipeline,
    uniforms: ShaderUniforms<Uniforms>,
    sources_bind_group: wgpu::BindGroup,
    normal_bind_group: wgpu::BindGroup,
    cornea_bind_group: wgpu::BindGroup,
    targets: ColorTargets,
}

impl Node for Lens {
    fn new(window: &Window) -> Self {
        let generator = NormalMapGenerator::new(&window);
        let device = window.device().borrow_mut();
        let queue = window.queue().borrow_mut();

        let uniforms = ShaderUniforms::new(&device, 
            Uniforms{
                lens_position: [0.0,0.0],
                active: 0,
                samplecount: 4,
                //depth_min: 200.0,  //XXX: was 1000.0 - 300.0,
                depth_min: 100.0,  //XXX: was 1000.0 - 300.0,
                //depth_max: 5000.0, //XXX: was 1000.0 + 0.0,
                depth_max: 1800.0, //XXX: was 1000.0 + 0.0,
                near_point: 0.0,
                far_point: f32::INFINITY,
                near_vision_factor: 0.0,
                far_vision_factor: 0.0,
                dir_calc_scale: 1.0,
                astigmatism_ecc_mm: 0.0,
                astigmatism_angle_deg: 0.0,
                eye_distance_center: 0.0,
                track_error: 0,
                _padding: 0,
            });

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Lens Shader"),
            source: wgpu::ShaderSource::Wgsl(concat!(
                include_str!("../vert.wgsl"),
                include_str!("lens_model.wgsl"),
                include_str!("mod.wgsl")).into()),
        });

        let (normal_layout, normal_bind_group) = placeholder_highp_texture(
            &device,
            &queue,
            Some("Lens-Normal Texture placeholder")
        ).unwrap().create_bind_group(&device);
        
        let (cornea_layout, cornea_bind_group) = placeholder_texture(
            &device,
            &queue,
            Some("Lens-Cornea Texture placeholder")
        ).unwrap().create_bind_group(&device);

        let (sources_bind_group_layout, sources_bind_group) = create_color_depth_sources_bind_group(&device, &queue, "Cataract");

        let pipeline = create_render_pipeline(
            &device,
            &[&shader, &shader],
            &["vs_main", "fs_main"],
            &[&uniforms.bind_group_layout, &sources_bind_group_layout, &normal_layout, &cornea_layout],
            &all_color_states(),
            None,
            Some("Lens Render Pipeline"));

        Lens {
            generator,
            pipeline,
            uniforms,
            sources_bind_group,
            normal_bind_group,
            cornea_bind_group,
            targets: ColorTargets::new(&device, "Lens"),
        }
    }

    fn negociate_slots(&mut self, window: &Window, slots: NodeSlots) -> NodeSlots {
        let slots = slots.to_color_depth_input(window).to_color_output(window);
        let device = window.device().borrow_mut();
        let queue = window.queue().borrow_mut();

        self.sources_bind_group = slots.as_all_source(&device);
        self.targets = slots.as_all_colors_target();

        let size = slots.output_size_f32();
        self.generator.generate(&device, &queue, size[0] as u32, size[1] as u32);
        (_, self.normal_bind_group) = self.generator.texture.create_bind_group(&device);

        slots
    }

    fn update_values(&mut self, _window: &Window, values: &ValueMap) {
        // default values
        self.uniforms.data.near_point = 0.0;
        self.uniforms.data.far_point = f32::INFINITY;
        self.uniforms.data.near_vision_factor = 0.0;
        self.uniforms.data.far_vision_factor = 0.0;
        self.uniforms.data.active = 0;

        if let Some(Value::Number(rays)) = values.get("rays") {
            self.uniforms.data.samplecount = *rays as i32;
        }

        if let Some(Value::Bool(true)) = values.get("presbyopia_onoff") {
            // near point is a parameter between 0 and 100 that is to be scaled to 0 - 1000
            if let Some(Value::Number(near_point)) = values.get("presbyopia_near_point") {
                self.uniforms.data.active = 1;
                self.uniforms.data.near_point = *near_point as f32;
                self.uniforms.data.near_vision_factor = 1.0;
            }
        }

        if let Some(Value::Bool(true)) = values.get("myopiahyperopia_onoff") {
            if let Some(Value::Number(mnh)) = values.get("myopiahyperopia_mnh") {
                self.uniforms.data.active = 1;
                // mnh represents a range of -3D to 3D
                let dioptres = ((mnh / 50.0 - 1.0) * 3.0) as f32;

                if dioptres < 0.0 {
                    // myopia
                    self.uniforms.data.far_point = -1000.0 / dioptres;
                    // u_near_point should not be farther than u_far_point
                    self.uniforms.data.near_point =
                        self.uniforms.data.near_point.min(self.uniforms.data.far_point);
                    let vision_factor = 1.0 - dioptres * DIOPTRES_SCALING;
                    self.uniforms.data.far_vision_factor =
                        self.uniforms.data.far_vision_factor.max(vision_factor as f32);
                } else if dioptres > 0.0 {
                    // hyperopia
                    let hyperopia_near_point = 1000.0 / (4.4 - dioptres);
                    self.uniforms.data.near_point =
                        self.uniforms.data.near_point.max(hyperopia_near_point);
                    let vision_factor = 1.0 + dioptres * DIOPTRES_SCALING;
                    self.uniforms.data.near_vision_factor =
                        self.uniforms.data.near_vision_factor.max(vision_factor as f32);
                }
            }
        }

        if let Some(Value::Number(astigmatism_dpt)) = values.get("astigmatism_dpt") {
            // dpt to eccentricity in mm: 0.2 mm ~ 1dpt
            // the actual formula is more complex but requires many parameters that are specific to an eye
            // since our values for the eye parametes are far from realistic, i would argue this is sufficient
            self.uniforms.data.astigmatism_ecc_mm = 0.2 * (*astigmatism_dpt as f32);
        }
        if let Some(Value::Number(astigmatism_angle_deg)) = values.get("astigmatism_angle_deg") {
            self.uniforms.data.astigmatism_angle_deg = *astigmatism_angle_deg as f32;
        }
        if let Some(Value::Number(eye_distance_center)) = values.get("eye_distance_center") {
            self.uniforms.data.eye_distance_center = *eye_distance_center as f32;
        }

    }

    fn input(&mut self, perspective: &EyePerspective, vis_param: &VisualizationParameters) -> EyePerspective {
        self.uniforms.data.dir_calc_scale = vis_param.dir_calc_scale;
        self.uniforms.data.depth_max = vis_param.test_depth_max;
        self.uniforms.data.depth_min = vis_param.test_depth_min;
        self.uniforms.data.lens_position[0] = vis_param.eye_position.0;
        self.uniforms.data.lens_position[1] = vis_param.eye_position.1;
        self.uniforms.data.track_error = vis_param.has_to_track_error() as i32;   
        perspective.clone()
    }

    fn render(&mut self, window: &window::Window, encoder: &mut CommandEncoder, screen: Option<&RenderTexture>) {
        self.uniforms.update(&window.queue().borrow_mut());
        
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Lens render_pass"),
            color_attachments: &self.targets.color_attachments(screen),
            depth_stencil_attachment: None,
        });
    
        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, &self.uniforms.bind_group, &[]);
        render_pass.set_bind_group(1, &self.sources_bind_group, &[]);
        render_pass.set_bind_group(2, &self.normal_bind_group, &[]);
        render_pass.set_bind_group(3, &self.cornea_bind_group, &[]);
        render_pass.draw(0..6, 0..1);
    }
}
