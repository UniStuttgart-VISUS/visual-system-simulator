use crate::pipeline::*;

gfx_defines! {
    pipeline pipe {
        s_y: gfx::TextureSampler<f32> = "s_y",
        s_u: gfx::TextureSampler<f32> = "s_u",
        s_v: gfx::TextureSampler<f32> = "s_v",
        rt_color: gfx::RenderTarget<ColorFormat> = "rt_color",
    }
}

pub struct YuvToRgb {
    pso: gfx::PipelineState<Resources, pipe::Meta>,
    pso_data: pipe::Data<Resources>,
}

impl Node for YuvToRgb {
    fn new(window: &Window) -> Self {
        let mut factory = window.factory().borrow_mut();

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

        YuvToRgb {
            pso,
            pso_data: pipe::Data {
                s_y: (srv.clone(), sampler.clone()),
                s_u: (srv.clone(), sampler.clone()),
                s_v: (srv, sampler),
                rt_color: rtv,
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
        self.pso_data.rt_color = target.clone();
        match source.0.expect("Source expected") {
            DeviceSource::Rgb { .. } => panic!("Unsupported source"),
            DeviceSource::RgbDepth { .. } => panic!("Unsupported source"),
            DeviceSource::Yuv { y, u, v, .. } => {
                self.pso_data.s_y = (y.clone(), factory.create_sampler_linear());
                self.pso_data.s_u = (u.clone(), factory.create_sampler_linear());
                self.pso_data.s_v = (v.clone(), factory.create_sampler_linear());
            }
        }
        (target_candidate.0, Some(target))
    }

    fn render(&mut self, window: &Window) {
        let mut encoder = window.encoder().borrow_mut();

        encoder.draw(&gfx::Slice::from_vertex_count(6), &self.pso, &self.pso_data);
    }
}