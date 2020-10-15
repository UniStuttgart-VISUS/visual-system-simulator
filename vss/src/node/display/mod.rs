use super::*;
use gfx;

gfx_defines! {
    pipeline pipe {
        u_stereo: gfx::Global<i32> = "u_stereo",
        u_resolution_in: gfx::Global<[f32; 2]> = "u_resolution_in",
        u_resolution_out: gfx::Global<[f32; 2]> = "u_resolution_out",
        s_source: gfx::TextureSampler<[f32; 4]> = "s_color",
        rt_color: gfx::RenderTarget<ColorFormat> = "rt_color",
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

        Display {
            pso,
            pso_data: pipe::Data {
                u_stereo: 0,
                u_resolution_in: [1.0, 1.0],
                u_resolution_out: [1.0, 1.0],
                s_source: (src, sampler),
                rt_color: dst,
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

        self.pso_data.u_resolution_out = [target_size.0 as f32, target_size.1 as f32];
        self.pso_data.rt_color = target.clone();
        match source.0.expect("Source expected") {
            NodeSource::Rgb {
                rgba8,
                width,
                height,
            } => {
                self.pso_data.u_resolution_in = [width as f32, height as f32];
                self.pso_data.s_source = (rgba8.clone(), factory.create_sampler_linear());
            }
            NodeSource::RgbDepth {
                rgba8,
                width,
                height,
                ..
            } => {
                self.pso_data.u_resolution_in = [width as f32, height as f32];
                self.pso_data.s_source = (rgba8.clone(), factory.create_sampler_linear());
            }
        }
        (target_candidate.0, Some(target))
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
        }
    }

    fn input(&mut self, _head: &Head, gaze: &Gaze) -> Gaze {
        let ratio = [
            self.pso_data.u_resolution_out[0] / self.pso_data.u_resolution_in[0],
            self.pso_data.u_resolution_out[1] / self.pso_data.u_resolution_in[1],
        ];
        let offset = [
            0.5 * (ratio[0] - ratio[1]).max(0.0),
            0.5 * (ratio[1] - ratio[0]).max(0.0),
        ];
        let scale = [
            ratio[0] / ratio[0].min(ratio[1]),
            ratio[1] / ratio[0].min(ratio[1]),
        ];

        Gaze {
            x: scale[0] * gaze.x - offset[0],
            y: scale[1] * gaze.y - offset[1],
        }
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
