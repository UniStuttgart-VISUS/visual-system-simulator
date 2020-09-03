use gfx;
use gfx::traits::FactoryExt;
use gfx::Factory;
use gfx_device_gl::CommandBuffer;
use gfx_device_gl::Resources;

use crate::devices::*;
use crate::pipeline::*;

gfx_defines! {
    pipeline pipe {
        u_stereo: gfx::Global<i32> = "u_stereo",
        s_source: gfx::TextureSampler<[f32; 4]> = "s_color",
        rt_color: gfx::RenderTarget<ColorFormat> = "rt_color",
    }
}

pub struct RgbDisplay {
    pso: gfx::PipelineState<Resources, pipe::Meta>,
    pso_data: pipe::Data<Resources>,
}

impl Pass for RgbDisplay {
    fn build(factory: &mut gfx_device_gl::Factory) -> Self {
        let pso = factory
            .create_pipeline_simple(
                &include_glsl!("shader.vert"),
                &include_glsl!("shader.frag"),
                pipe::new(),
            )
            .unwrap();

        let sampler = factory.create_sampler_linear();
        let (_, src, dst) = factory.create_render_target(1, 1).unwrap();

        RgbDisplay {
            pso,
            pso_data: pipe::Data {
                u_stereo: 0,
                s_source: (src, sampler),
                rt_color: dst,
            },
        }
    }

    fn update_io(
        &mut self,
        target: &DeviceTarget,
        _target_size: (u32, u32),
        source: &DeviceSource,
        source_sampler: &gfx::handle::Sampler<Resources>,
        _source_size: (u32, u32),
    ) {
        self.pso_data.rt_color = target.clone();
        match source {
            DeviceSource::Rgb { rgba8 } => {
                self.pso_data.s_source = (rgba8.clone(), source_sampler.clone());
            }
            DeviceSource::RgbDepth { rgba8, d: _ } => {
                self.pso_data.s_source = (rgba8.clone(), source_sampler.clone());
            }
            DeviceSource::Yuv { .. } => panic!("Unsupported source"),
        }
    }

    fn update_params(&mut self, _factory: &mut gfx_device_gl::Factory, values: &ValueMap) {
        self.pso_data.u_stereo = if values
            .get("split_screen_switch")
            .unwrap_or(&Value::Bool(false))
            .as_bool()
            .unwrap_or(false)
        {
            1
        } else {
            0
        }
    }

    fn render(&mut self, encoder: &mut gfx::Encoder<Resources, CommandBuffer>, _gaze: &DeviceGaze) {
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
