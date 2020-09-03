use gfx;
use gfx::traits::FactoryExt;
use gfx::Factory;
use gfx_device_gl::CommandBuffer;
use gfx_device_gl::Resources;

use crate::devices::*;
use crate::pipeline::*;

gfx_defines! {
    pipeline pipe {
        u_resolution_in: gfx::Global<[f32;2]> = "u_resolution_in",
        u_resolution_out: gfx::Global<[f32;2]> = "u_resolution_out",
        u_rotation: gfx::Global<f32> = "u_rotation",
        s_y: gfx::TextureSampler<f32> = "s_y",
        s_u: gfx::TextureSampler<f32> = "s_u",
        s_v: gfx::TextureSampler<f32> = "s_v",
        rt_color: gfx::RenderTarget<ColorFormat> = "rt_color",
    }
}

pub struct Yuv420Rgb {
    pso: gfx::PipelineState<Resources, pipe::Meta>,
    pso_data: pipe::Data<Resources>,
}

impl Pass for Yuv420Rgb {
    fn build(factory: &mut gfx_device_gl::Factory) -> Self {
        let pso = factory
            .create_pipeline_simple(
                &include_glsl!("shader.vert"),
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

        Yuv420Rgb {
            pso,
            pso_data: pipe::Data {
                u_resolution_in: [1.0 as f32, 1.0 as f32],
                u_resolution_out: [1.0 as f32, 1.0 as f32],
                u_rotation: 0.0 as f32,
                s_y: (srv.clone(), sampler.clone()),
                s_u: (srv.clone(), sampler.clone()),
                s_v: (srv, sampler),
                rt_color: rtv,
            },
        }
    }

    fn update_io(
        &mut self,
        target: &DeviceTarget,
        target_size: (u32, u32),
        source: &DeviceSource,
        source_sampler: &gfx::handle::Sampler<Resources>,
        source_size: (u32, u32),
    ) {
        self.pso_data.rt_color = target.clone();
        self.pso_data.u_resolution_out = [target_size.0 as f32, target_size.1 as f32];
        match source {
            DeviceSource::Rgb { .. } => panic!("Unsupported source"),
            DeviceSource::RgbDepth { .. } => panic!("Unsupported source"),
            DeviceSource::Yuv { y, u, v } => {
                self.pso_data.s_y = (y.clone(), source_sampler.clone());
                self.pso_data.s_u = (u.clone(), source_sampler.clone());
                self.pso_data.s_v = (v.clone(), source_sampler.clone());
            }
        }
        self.pso_data.u_resolution_in = [source_size.0 as f32, source_size.1 as f32];
    }

    fn update_params(&mut self, _factory: &mut gfx_device_gl::Factory, params: &ValueMap) {
        if let Some(Value::Number(rotation)) = params.get("rotation") {
            self.pso_data.u_rotation = -rotation.to_radians() as f32;
        }
    }

    fn render(&mut self, encoder: &mut gfx::Encoder<Resources, CommandBuffer>, _gaze: &DeviceGaze) {
        encoder.draw(&gfx::Slice::from_vertex_count(6), &self.pso, &self.pso_data);
    }
}
