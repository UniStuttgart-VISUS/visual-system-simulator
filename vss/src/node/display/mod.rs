use super::*;
use gfx;
use cgmath::Matrix4;
use cgmath::Rad;

gfx_defines! {
    pipeline pipe {
        u_stereo: gfx::Global<i32> = "u_stereo",
        u_resolution_in: gfx::Global<[f32; 2]> = "u_resolution_in",
        u_resolution_out: gfx::Global<[f32; 2]> = "u_resolution_out",
        s_source: gfx::TextureSampler<[f32; 4]> = "s_color",
        rt_color: gfx::RenderTarget<ColorFormat> = "rt_color",
        u_vis_type: gfx::Global<i32> = "u_vis_type",
        u_heat_scale: gfx::Global<f32> = "u_heat_scale",
        u_dir_calc_scale: gfx::Global<f32> = "u_dir_calc_scale",
        s_deflection: gfx::TextureSampler<[f32; 4]> = "s_deflection",
        s_color_change: gfx::TextureSampler<[f32; 4]> = "s_color_change",
        s_color_uncertainty: gfx::TextureSampler<[f32; 4]> = "s_color_uncertainty",
        s_original: gfx::TextureSampler<[f32; 4]> = "s_original",
        s_covariances: gfx::TextureSampler<[f32; 4]> = "s_covariances",
        u_flow_idx: gfx::Global<i32> = "u_flow_idx",
        u_hive_rotation: gfx::Global<[[f32; 4];4]> = "u_hive_rotation",
        u_hive_position: gfx::Global<[f32; 2]> = "u_hive_position",
        u_hive_visible: gfx::Global<i32> = "u_hive_visible",

        u_base_image: gfx::Global<i32> = "u_base_image",
        u_combination_function: gfx::Global<i32> = "u_combination_function",
        u_mix_type: gfx::Global<i32> = "u_mix_type",
        u_colormap_type: gfx::Global<i32> = "u_colormap_type",
    }
}



pub struct Display {
    pso: gfx::PipelineState<Resources, pipe::Meta>,
    pso_data: pipe::Data<Resources>,
    hive_rot: Matrix4<f32>
}

impl Node for Display {
    fn new(window: &Window) -> Self {
        let mut factory = window.factory().borrow_mut();

        let pso = factory
            .create_pipeline_simple(
                &include_glsl!("mod.vert"),
                &include_glsl!("mod.frag"),
                pipe::new(),
            )
            .unwrap();

        let sampler = factory.create_sampler_linear();
        let (_, src, dst) = factory.create_render_target(1, 1).unwrap();

        // add deflection view
        let (_, s_deflection, _): (
            _,
            _,
            gfx::handle::RenderTargetView<gfx_device_gl::Resources, [f32; 4]>,
        ) = factory.create_render_target(1, 1).unwrap();
        let (_, s_color_change, _): (
            _,
            _,
            gfx::handle::RenderTargetView<gfx_device_gl::Resources, [f32; 4]>,
        ) = factory.create_render_target(1, 1).unwrap();
        let (_, s_color_uncertainty, _): (
            _,
            _,
            gfx::handle::RenderTargetView<gfx_device_gl::Resources, [f32; 4]>,
        ) = factory.create_render_target(1, 1).unwrap();
        let (_, s_original, _): (
            _,
            _,
            gfx::handle::RenderTargetView<gfx_device_gl::Resources, [f32; 4]>,
        ) = factory.create_render_target(1, 1).unwrap();

        let (_, s_covariances, _):(
            _,
            _,
            gfx::handle::RenderTargetView<gfx_device_gl::Resources, [f32; 4]>,
        ) = factory.create_render_target(1, 1).unwrap();

        Display {
            pso,
            pso_data: pipe::Data {
                u_stereo: 0,
                u_resolution_in: [1.0, 1.0],
                u_resolution_out: [1.0, 1.0],
                s_source: (src, sampler.clone()),
                s_deflection: (s_deflection, sampler.clone()),
                s_color_change:(s_color_change, sampler.clone()),
                s_color_uncertainty:(s_color_uncertainty, sampler.clone()),
                s_original:(s_original, sampler.clone()),
                s_covariances: (s_covariances, sampler.clone()),
                rt_color: dst,
                u_vis_type: 0,
                u_heat_scale: 1.0,
                u_dir_calc_scale: 1.0,
                u_flow_idx: 0,
                u_hive_rotation: [[0.0; 4]; 4],
                u_hive_position: [0.0; 2],
                u_hive_visible: 0,
                u_base_image: 0,
                u_combination_function: 0,
                u_mix_type: 0,
                u_colormap_type: 0
            },
            hive_rot: Matrix4::from_angle_x(Rad(0.0))
        }
    }

    fn negociate_slots(&mut self, window: &Window, slots: NodeSlots) -> NodeSlots {
        let slots = slots.to_color_input(window).to_color_output(window);
        self.pso_data.u_resolution_in = slots.input_size_f32();
        self.pso_data.u_resolution_out = slots.output_size_f32();
        self.pso_data.s_source = slots.as_color_view();
        self.pso_data.s_deflection = slots.as_deflection_view();
        self.pso_data.s_color_change = slots.as_color_change_view();
        self.pso_data.s_color_uncertainty = slots.as_color_uncertainty_view();
        self.pso_data.s_covariances = slots.as_covariances_view();
        
        self.pso_data.rt_color = slots.as_color();

        slots
    }

    fn negociate_slots_wk(&mut self, window: &Window, slots: NodeSlots, well_known: &WellKnownSlots) -> NodeSlots{
        self.pso_data.s_original = well_known.get_original().expect("Nah, no original image?");
        self.negociate_slots(window, slots)
    }


    fn update_values(&mut self, _window: &Window, values: &ValueMap) {
        self.pso_data.u_stereo = if values
            .get("split_screen_switch")
            .unwrap_or(&Value::Bool(false))
            .as_bool()
            .unwrap_or(false)
        {
            1
        } else {
            0
        };

        self.pso_data.u_flow_idx = values.get("flow_id").unwrap_or(&Value::Number(0.0)).as_f64().unwrap_or(0.0) as i32;
    }

    fn input(&mut self, perspective: &EyePerspective, vis_param: &VisualizationParameters) -> EyePerspective {
        //self.pso_data.u_vis_type = ((vis_param.vis_type) as u32) as i32;
        self.pso_data.u_heat_scale = vis_param.heat_scale;
        self.pso_data.u_dir_calc_scale = vis_param.dir_calc_scale;
        self.pso_data.u_flow_idx = vis_param.eye_idx as i32;
        self.pso_data.u_hive_position[0] = vis_param.highlight_position.0 as f32;
        self.pso_data.u_hive_position[1] = vis_param.highlight_position.1 as f32;
        self.pso_data.u_hive_visible = vis_param.bees_visible as i32;

        self.pso_data.u_base_image =  ((vis_param.vis_type.base_image) as u32) as i32;
        self.pso_data.u_combination_function = ((vis_param.vis_type.combination_function) as u32) as i32;
        self.pso_data.u_mix_type = ((vis_param.vis_type.mix_type) as u32) as i32;
        self.pso_data.u_colormap_type = ((vis_param.vis_type.color_map_type) as u32) as i32;

        perspective.clone()
    }

    fn render(&mut self, window: &Window) {

        let speed = 4.0;

        self.hive_rot= self.hive_rot*Matrix4::from_angle_x(Rad(       speed * window.delta_t()/1_000_000.0));
        self.hive_rot= self.hive_rot*Matrix4::from_angle_y(Rad( 0.7 * speed * window.delta_t()/1_000_000.0));
        self.hive_rot= self.hive_rot*Matrix4::from_angle_z(Rad( 0.2 * speed * window.delta_t()/1_000_000.0));

        self.pso_data.u_hive_rotation = self.hive_rot.into();

        let mut encoder = window.encoder().borrow_mut();

        if self.pso_data.u_stereo == 0 {
            encoder.draw(&gfx::Slice::from_vertex_count(6), &self.pso, &self.pso_data);
        } else {
            encoder.draw(
                &gfx::Slice::from_vertex_count(12),
                &self.pso,
                &self.pso_data,
            );
        }
    }
}
