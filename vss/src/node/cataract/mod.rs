use super::*;
use gfx;

gfx_defines! {
    pipeline pipe {
        u_active: gfx::Global<i32> = "u_active",
        u_resolution: gfx::Global<[f32; 2]> = "u_resolution",
        u_blur_factor: gfx::Global<f32> = "u_blur_factor",
        u_contrast_factor: gfx::Global<f32> = "u_contrast_factor",
        s_color: gfx::TextureSampler<[f32; 4]> = "s_color",
        //s_depth: gfx::TextureSampler<f32> = "s_depth",
        rt_color: gfx::RenderTarget<ColorFormat> = "rt_color",
        //rt_depth: gfx::RenderTarget<DepthFormat> = "rt_depth",
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
        //let (_, depth_view) =
        //    load_single_channel_texture_from_bytes(&mut factory, &[0; 4], 1, 1).unwrap();
        let (_, _, rt_color) = factory.create_render_target(1, 1).unwrap();
        //let (_, _, rt_depth) = factory.create_render_target(1, 1).unwrap();

        Cataract {
            pso,
            pso_data: pipe::Data {
                u_active: 0,
                u_resolution: [0.0, 0.0],
                u_blur_factor: 0.0,
                u_contrast_factor: 0.0,
                s_color: (color_view, sampler.clone()),
                //s_depth: (depth_view, sampler),
                rt_color,
                //rt_depth,
            },
        }
    }

    fn update_io(
        &mut self,
        window: &Window,
        source: (Option<NodeSource>, Option<NodeTarget>),
        target_candidate: (Option<NodeSource>, Option<NodeTarget>),
    ) -> (Option<NodeSource>, Option<NodeTarget>) {
        let mut factory = window.factory().borrow_mut();
        let target = target_candidate.1.expect("Render target expected");
        let target_size = target.get_dimensions();
        self.pso_data.u_resolution = [target_size.0 as f32, target_size.1 as f32];
        self.pso_data.rt_color = target.clone();
        match source.0.expect("Source expected") {
            NodeSource::Rgb { color, .. } => {
                self.pso_data.s_color = (color.clone(), factory.create_sampler_linear());
            }
            NodeSource::RgbDepth { color, depth, .. } => {
                self.pso_data.s_color = (color.clone(), factory.create_sampler_linear());
                //self.pso_data.s_depth = (depth.clone(), factory.create_sampler_linear());
            }
        }
        (target_candidate.0, Some(target))
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
}
