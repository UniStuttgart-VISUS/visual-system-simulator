use super::*;
use gfx;
use gfx::format::Rgba32F;

gfx_defines! {
    pipeline pipe {
        u_active: gfx::Global<i32> = "u_active",
        u_resolution: gfx::Global<[f32; 2]> = "u_resolution",
        u_blur_factor: gfx::Global<f32> = "u_blur_factor",
        u_contrast_factor: gfx::Global<f32> = "u_contrast_factor",
        s_color: gfx::TextureSampler<[f32; 4]> = "s_color",
        rt_color: gfx::RenderTarget<ColorFormat> = "rt_color",
        s_depth: gfx::TextureSampler<f32> = "s_depth",
        rt_depth: gfx::RenderTarget<DepthFormat> = "rt_depth",
        s_deflection: gfx::TextureSampler<[f32; 4]> = "s_deflection",
        rt_deflection: gfx::RenderTarget<Rgba32F> = "rt_deflection",
        s_color_change: gfx::TextureSampler<[f32; 4]> = "s_color_change",
        rt_color_change: gfx::RenderTarget<Rgba32F> = "rt_color_change",
        s_color_uncertainty: gfx::TextureSampler<[f32; 4]> = "s_color_uncertainty",
        rt_color_uncertainty: gfx::RenderTarget<Rgba32F> = "rt_color_uncertainty",
        s_covariances: gfx::TextureSampler<[f32; 4]> = "s_covariances",
        rt_covariances: gfx::RenderTarget<Rgba32F> = "rt_covariances",
        u_track_error: gfx::Global<i32> = "u_track_error",
    }
}

pub struct Cataract {
    pso: gfx::PipelineState<Resources, pipe::Meta>,
    pso_data: pipe::Data<Resources>,
}

impl Node for Cataract {
    fn new(window: &Window) -> Self {
        let mut factory = window.factory().borrow_mut();
        let pso = factory
            .create_pipeline_simple(
                &include_glsl!("../mod.vert"),
                &include_glsl!("mod.frag"),
                pipe::new(),
            )
            .unwrap();
        let sampler = factory.create_sampler_linear();
        let (_, color_view) = load_texture_from_bytes(&mut factory, &[0; 4], 1, 1).unwrap();
        let (_, depth_view) =
            load_single_channel_texture_from_bytes(&mut factory, &[0; 4], 1, 1).unwrap();
        let (_, _, rt_color) = factory.create_render_target(1, 1).unwrap();
        let (_, _, rt_depth) = factory.create_render_target(1, 1).unwrap();
        let (_, s_deflection, rt_deflection) = factory.create_render_target(1, 1).unwrap();
        let (_, s_color_change, rt_color_change) = factory.create_render_target(1, 1).unwrap();
        let (_, s_color_uncertainty, rt_color_uncertainty) = factory.create_render_target(1, 1).unwrap();
        let (_, s_covariances, rt_covariances) = factory.create_render_target(1, 1).unwrap();


        Cataract {
            pso,
            pso_data: pipe::Data {
                u_active: 0,
                u_resolution: [0.0, 0.0],
                u_blur_factor: 0.0,
                u_contrast_factor: 0.0,
                s_color: (color_view, sampler.clone()),
                s_depth: (depth_view, sampler.clone()),
                rt_color,
                rt_depth,
                s_deflection:(s_deflection, sampler.clone()),
                rt_deflection,
                s_color_change:(s_color_change, sampler.clone()),
                rt_color_change,
                s_color_uncertainty:(s_color_uncertainty, sampler.clone()),
                rt_color_uncertainty,
                s_covariances: (s_covariances, sampler.clone()),
                rt_covariances,
                u_track_error: 0
            },
        }
    }

    fn negociate_slots(&mut self, window: &Window, slots: NodeSlots) -> NodeSlots {
        let slots = slots
            .to_color_depth_input(window)
            .to_color_depth_output(window);
        self.pso_data.u_resolution = slots.output_size_f32();
        let (color_view, depth_view) = slots.as_color_depth_view();


        self.pso_data.s_color = color_view;
        self.pso_data.s_depth = depth_view;
        let (color, depth) = slots.as_color_depth();
        self.pso_data.rt_color = color;
        self.pso_data.rt_depth = depth;
        self.pso_data.s_deflection = slots.as_deflection_view();
        self.pso_data.rt_deflection = slots.as_deflection();
        self.pso_data.s_color_change = slots.as_color_change_view();
        self.pso_data.rt_color_change = slots.as_color_change();  
        self.pso_data.s_color_uncertainty = slots.as_color_uncertainty_view();
        self.pso_data.rt_color_uncertainty = slots.as_color_uncertainty();
        self.pso_data.s_covariances = slots.as_covariances_view();
        self.pso_data.rt_covariances = slots.as_covariances();

        slots
    }
    fn negociate_slots_wk(&mut self, window: &Window, slots: NodeSlots, well_known: &WellKnownSlots) -> NodeSlots{
        let slots = self.negociate_slots(window, slots);
        well_known.set_original(slots.as_color_depth_view().0);
        slots
    }

    fn update_values(&mut self, _window: &Window, values: &ValueMap) {
        if let Some(Value::Bool(true)) = values.get("ct_onoff") {
            self.pso_data.u_active = 1;
            if let Some(Value::Number(ct_blur_factor)) = values.get("ct_blur_factor") {
                // ct_blur_factor is between 0 and 100
                self.pso_data.u_blur_factor = (*ct_blur_factor as f32) / 100.0;
            }
            if let Some(Value::Number(ct_contrast_factor)) = values.get("ct_contrast_factor") {
                //  ct_contrast_factor is between 0 and 100
                self.pso_data.u_contrast_factor = (*ct_contrast_factor as f32) / 100.0;
            }
        } else {
            self.pso_data.u_active = 0;
            self.pso_data.u_blur_factor = 0.0;
            self.pso_data.u_contrast_factor = 0.0;
        }
    }

    fn render(&mut self, window: &Window) {
        let mut encoder = window.encoder().borrow_mut();
        encoder.draw(&gfx::Slice::from_vertex_count(6), &self.pso, &self.pso_data);
    }
    fn input(&mut self, perspective: &EyePerspective, vis_param: &VisualizationParameters) -> EyePerspective {        
        self.pso_data.u_track_error = vis_param.has_to_track_error() as i32;        
        perspective.clone()
    }
}
