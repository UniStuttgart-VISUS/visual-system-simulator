use gfx;
use gfx::traits::FactoryExt;
use gfx_device_gl::CommandBuffer;
use gfx_device_gl::Resources;

use crate::devices::*;
use crate::pipeline::*;

gfx_defines! {
    pipeline pipe {
        u_stereo: gfx::Global<i32> = "u_stereo",
        s_y: gfx::TextureSampler<f32> = "s_y",
        s_u: gfx::TextureSampler<f32> = "s_u",
        s_v: gfx::TextureSampler<f32> = "s_v",
        rt_color: gfx::RenderTarget<ColorFormat> = "rt_color",
    }
}

pub struct YuvRgb {
    pso: gfx::PipelineState<Resources, pipe::Meta>,
    pso_data: pipe::Data<Resources>,
}

impl YuvRgb {
    pub fn new<F: gfx::Factory<Resources>>(factory: &mut F) -> Self {
        let pso = factory
            .create_pipeline_simple(
                &include_glsl!("../shader.vert"),
                &include_glsl!("shader.frag"),
                pipe::new(),
            )
            .unwrap();

        let sampler = factory.create_sampler_linear();
        let (_, srv, _): (
            _,
            _,
            gfx::handle::RenderTargetView<gfx_device_gl::Resources, f32>,
        ) = factory.create_render_target(1, 1).unwrap();
        let (_, _, rtv) = factory.create_render_target(1, 1).unwrap();

        YuvRgb {
            pso,
            pso_data: pipe::Data {
                u_stereo: 0,
                s_y: (srv.clone(), sampler.clone()),
                s_u: (srv.clone(), sampler.clone()),
                s_v: (srv, sampler),
                rt_color: rtv,
            },
        }
    }
}

impl Pass for YuvRgb {
    fn update_io(
        &mut self,
        target: &DeviceTarget,
        _target_size: (u32, u32),
        source: &DeviceSource,
        source_sampler: &gfx::handle::Sampler<Resources>,
        _source_size: (u32, u32),
        stereo: bool,
    ) {
        self.pso_data.u_stereo = if stereo { 1 } else { 0 };
        self.pso_data.rt_color = target.clone();
        match source {
            DeviceSource::Rgb { .. } => panic!("Unsupported source"),
            DeviceSource::RgbDepth { .. } => panic!("Unsupported source"),
            DeviceSource::Yuv { y, u, v } => {
                self.pso_data.s_y = (y.clone(), source_sampler.clone());
                self.pso_data.s_u = (u.clone(), source_sampler.clone());
                self.pso_data.s_v = (v.clone(), source_sampler.clone());
            }
        }
    }

    fn update_params(&mut self, _factory: &mut gfx_device_gl::Factory, _values: &ValueMap) {}

    fn render(&mut self, encoder: &mut gfx::Encoder<Resources, CommandBuffer>, _gaze: &DeviceGaze) {
        encoder.draw(&gfx::Slice::from_vertex_count(6), &self.pso, &self.pso_data);
    }
}
