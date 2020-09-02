use gfx;
use gfx::traits::FactoryExt;
use gfx_device_gl::CommandBuffer;
use gfx_device_gl::Resources;

use crate::devices::*;
use crate::pipeline::*;

gfx_defines! {
    pipeline pipe {
        u_stereo: gfx::Global<i32> = "u_stereo",
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

impl Cataract {
    pub fn new<F: gfx::Factory<Resources>>(factory: &mut F) -> Cataract {
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
                u_stereo: 0,
                u_active: 0,
                u_resolution: [1.0, 1.0],
                u_blur_factor: 0.0,
                u_contrast_factor: 0.0,
                s_color: (src, sampler),
                rt_color: dst,
            },
        }
    }
}

impl Pass for Cataract {
    fn update_io(
        &mut self,
        target: &DeviceTarget,
        _target_size: (u32, u32),
        source: &DeviceSource,
        source_sampler: &gfx::handle::Sampler<Resources>,
        source_size: (u32, u32),
        stereo: bool,
    ) {
        self.pso_data.u_stereo = if stereo { 1 } else { 0 };
        self.pso_data.rt_color = target.clone();
        match source {
            DeviceSource::Rgb { rgba8 } => {
                self.pso_data.s_color = (rgba8.clone(), source_sampler.clone());
            }
            DeviceSource::RgbDepth { rgba8, d: _ } => {
                self.pso_data.s_color = (rgba8.clone(), source_sampler.clone());
            }
            DeviceSource::Yuv { .. } => panic!("Unsupported source"),
        }
        self.pso_data.u_resolution = [source_size.0 as f32, source_size.1 as f32];
    }

    fn update_params(&mut self, _factory: &mut gfx_device_gl::Factory, params: &ValueMap) {
        if let Some(Value::Bool(true)) = params.get("ct_onoff") {
            self.pso_data.u_active = 1;
            if let Some(Value::Number(ct_blur_factor)) = params.get("ct_blur_factor") {
                // ct_blur_factor is between 0 and 100
                self.pso_data.u_blur_factor = (*ct_blur_factor as f32) / 100.0;
            }
            if let Some(Value::Number(ct_contrast_factor)) = params.get("ct_contrast_factor") {
                //  ct_contrast_factor is between 0 and 100
                self.pso_data.u_contrast_factor = (*ct_contrast_factor as f32) / 100.0;
            }
        } else {
            self.pso_data.u_active = 0;
            self.pso_data.u_blur_factor = 0.0;
            self.pso_data.u_contrast_factor = 0.0;
        }
    }

    fn render(&mut self, encoder: &mut gfx::Encoder<Resources, CommandBuffer>, _: &DeviceGaze) {
        encoder.draw(&gfx::Slice::from_vertex_count(6), &self.pso, &self.pso_data);
    }
}
