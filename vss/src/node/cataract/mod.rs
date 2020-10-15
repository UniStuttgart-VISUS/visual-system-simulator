use gfx;
use gfx_device_gl::Resources;

use crate::*;

gfx_defines! {
    pipeline pipe {
        u_active: gfx::Global<i32> = "u_active",
        u_resolution: gfx::Global<[f32; 2]> = "u_resolution",
        u_blur_factor: gfx::Global<f32> = "u_blur_factor",
        u_contrast_factor: gfx::Global<f32> = "u_contrast_factor",
        s_color: gfx::TextureSampler<[f32; 4]> = "s_color",
        rt_color: gfx::RenderTarget<ColorFormat> = "rt_color",
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
                &include_glsl!("../shader.vert"),
                &include_glsl!("shader.frag"),
                pipe::new(),
            )
            .unwrap();
        let (_, src, dst) = factory.create_render_target(1, 1).unwrap();
        let sampler = factory.create_sampler_linear();

        Cataract {
            pso,
            pso_data: pipe::Data {
                u_active: 0,
                u_resolution: [0.0, 0.0],
                u_blur_factor: 0.0,
                u_contrast_factor: 0.0,
                s_color: (src, sampler),
                rt_color: dst,
            },
        }
    }

    fn update_io(
        &mut self,
        window: &Window,
        source: (Option<DeviceSource>, Option<DeviceTarget>),
        target_candidate: (Option<DeviceSource>, Option<DeviceTarget>),
    ) -> (Option<DeviceSource>, Option<DeviceTarget>) {
        let mut factory = window.factory().borrow_mut();
        let target = target_candidate.1.expect("Render target expected");
        let target_size = target.get_dimensions();
        self.pso_data.u_resolution = [target_size.0 as f32, target_size.1 as f32];
        self.pso_data.rt_color = target.clone();
        match source.0.expect("Source expected") {
            DeviceSource::Rgb { rgba8, .. } => {
                self.pso_data.s_color = (rgba8.clone(), factory.create_sampler_linear());
            }
            DeviceSource::RgbDepth { rgba8, .. } => {
                self.pso_data.s_color = (rgba8.clone(), factory.create_sampler_linear());
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
