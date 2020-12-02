use super::*;
use gfx;

gfx_defines! {
    pipeline pipe {
        u_resolution_out: gfx::Global<[f32; 2]> = "u_resolution_out",
        u_flow_index: gfx::Global<u32> = "u_flow_index",
        s_source: gfx::TextureSampler<[f32; 4]> = "s_color",
        rt_color: gfx::RenderTarget<ColorFormat> = "rt_color",
    }
}

pub struct Compositor {
    pso: gfx::PipelineState<Resources, pipe::Meta>,
    pso_data: pipe::Data<Resources>,
}

impl Node for Compositor {
    fn new(window: &Window) -> Self {
        let mut factory = window.factory().borrow_mut();

        let pso = factory
            .create_pipeline_simple(
                &include_glsl!("compositor.vert"),
                &include_glsl!("compositor.frag"),
                pipe::new(),
            )
            .unwrap();

        let sampler = factory.create_sampler_linear();
        let (_, src, dst) = factory.create_render_target(1, 1).unwrap();

        Compositor {
            pso,
            pso_data: pipe::Data {
                u_resolution_out: [1.0, 1.0],
                u_flow_index: 0,
                s_source: (src, sampler),
                rt_color: dst,
            },
        }
    }

    fn negociate_slots(&mut self, window: &Window, slots: NodeSlots) -> NodeSlots {
        let slots = slots.to_color_input(window).to_color_output(window);

        self.pso_data.u_resolution_out = slots.output_size_f32();
        self.pso_data.s_source = slots.as_color_view();
        self.pso_data.rt_color = slots.as_color();
        slots
    }

    fn input(&mut self, _head: &Head, gaze: &Gaze, _vis_param: &VisualizationParameters, flow_index: usize) -> Gaze {
        if flow_index < 4 {
            self.pso_data.u_flow_index = flow_index as u32;
        }
        gaze.clone()
    }

    fn render(&mut self, window: &Window) {
        let mut encoder = window.encoder().borrow_mut();
        encoder.draw(&gfx::Slice::from_vertex_count(6), &self.pso, &self.pso_data);
    }
}
