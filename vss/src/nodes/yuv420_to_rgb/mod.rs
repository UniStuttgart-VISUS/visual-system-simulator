use gfx;
use gfx::traits::FactoryExt;
use gfx::Factory;
use gfx_device_gl::CommandBuffer;
use gfx_device_gl::Resources;

use crate::pipeline::*;

gfx_defines! {
    pipeline pipe {
        u_aspect_in: gfx::Global<f32> = "u_aspect_in",
        u_aspect_out: gfx::Global<f32> = "u_aspect_out",
        u_rotation: gfx::Global<f32> = "u_rotation",
        s_y: gfx::TextureSampler<f32> = "s_y",
        s_u: gfx::TextureSampler<f32> = "s_u",
        s_v: gfx::TextureSampler<f32> = "s_v",
        rt_color: gfx::RenderTarget<ColorFormat> = "rt_color",
    }
}

pub struct Yuv420ToRgb {
    pso: gfx::PipelineState<Resources, pipe::Meta>,
    pso_data: pipe::Data<Resources>,
}

impl Node for Yuv420ToRgb {
    fn new(factory: &mut gfx_device_gl::Factory) -> Self {
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

        Yuv420ToRgb {
            pso,
            pso_data: pipe::Data {
                u_aspect_in: 1.0,
                u_aspect_out: 1.0,
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
        factory: &mut gfx_device_gl::Factory,
        source: Option<DeviceSource>,
        target_candidate: (Option<DeviceSource>, Option<DeviceTarget>),
    ) -> (Option<DeviceSource>, Option<DeviceTarget>) {
        let source = source.expect("Source expected");
        let target = target_candidate.1.expect("Render target expected");
        let target_size = target.get_dimensions();
        self.pso_data.rt_color = target.clone();
        self.pso_data.u_aspect_out = target_size.0 as f32 / target_size.1 as f32;
        match source {
            DeviceSource::Rgb { .. } => panic!("Unsupported source"),
            DeviceSource::RgbDepth { .. } => panic!("Unsupported source"),
            DeviceSource::Yuv {
                width,
                height,
                y,
                u,
                v,
                ..
            } => {
                self.pso_data.u_aspect_in = width as f32 / height as f32;
                self.pso_data.s_y = (y.clone(), factory.create_sampler_linear());
                self.pso_data.s_u = (u.clone(), factory.create_sampler_linear());
                self.pso_data.s_v = (v.clone(), factory.create_sampler_linear());
            }
        }
        (target_candidate.0, Some(target))
    }

    fn update_values(&mut self, _factory: &mut gfx_device_gl::Factory, values: &ValueMap) {
        if let Some(Value::Number(rotation)) = values.get("rotation") {
            self.pso_data.u_rotation = -rotation.to_radians() as f32;
        }
    }

    fn render(&mut self, encoder: &mut gfx::Encoder<Resources, CommandBuffer>) {
        encoder.draw(&gfx::Slice::from_vertex_count(6), &self.pso, &self.pso_data);
    }
}
