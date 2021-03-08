use super::*;
use gfx;

gfx_defines! {
    pipeline pipe {
        u_stereo: gfx::Global<i32> = "u_stereo",
        u_resolution_in: gfx::Global<[f32; 2]> = "u_resolution_in",
        u_resolution_out: gfx::Global<[f32; 2]> = "u_resolution_out",
        s_source: gfx::TextureSampler<[f32; 4]> = "s_color",
        rt_color: gfx::RenderTarget<ColorFormat> = "rt_color",
        u_vis_type: gfx::Global<i32> = "u_vis_type",
        u_heat_scale: gfx::Global<f32> = "u_heat_scale",
        s_deflection: gfx::TextureSampler<[f32; 4]> = "s_deflection",
        s_color_change: gfx::TextureSampler<[f32; 4]> = "s_color_change",
        s_color_uncertainty: gfx::TextureSampler<[f32; 4]> = "s_color_uncertainty",
        s_original: gfx::TextureSampler<[f32; 4]> = "s_original",
        s_covariances: gfx::TextureSampler<[f32; 4]> = "s_covariances",
        u_flow_idx: gfx::Global<i32> = "u_flow_idx",
    }
}



pub struct Display {
    pso: gfx::PipelineState<Resources, pipe::Meta>,
    pso_data: pipe::Data<Resources>,
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
                u_flow_idx: 0
            },
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
        self.pso_data.u_vis_type = ((vis_param.vis_type) as u32) as i32;
        self.pso_data.u_heat_scale = vis_param.heat_scale;
        self.pso_data.u_flow_idx = vis_param.eye_idx as i32;
        perspective.clone()
    }

    fn render(&mut self, window: &Window) {
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
