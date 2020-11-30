use super::*;
use gfx;

gfx_defines! {
    pipeline pipe {
        u_view_context_l: gfx::Global<[[f32; 4];4]> = "u_view_context_l",
        u_view_context_r: gfx::Global<[[f32; 4];4]> = "u_view_context_r",
        u_view: gfx::Global<[[f32; 4];4]> = "u_view",
        u_proj: gfx::Global<[[f32; 4];4]> = "u_proj",
        s_color: gfx::TextureSampler<[f32; 4]> = "s_color",
        s_depth: gfx::TextureSampler<f32> = "s_depth",
        rt_color: gfx::RenderTarget<ColorFormat> = "rt_color",
        rt_depth: gfx::RenderTarget<DepthFormat> = "rt_depth",
    }
}

pub struct Seperator {
    pso: gfx::PipelineState<Resources, pipe::Meta>,
    pso_data: pipe::Data<Resources>,
}

impl Node for Seperator {
    fn new(window: &Window) -> Self {
        let mut factory = window.factory().borrow_mut();

        let pso = factory
            .create_pipeline_simple(
                &include_glsl!("../mod.vert"),
                &include_glsl!("seperator.frag"),
                pipe::new(),
            )
            .unwrap();

            let sampler = factory.create_sampler_linear();
            let (_, color_view) = load_texture_from_bytes(&mut factory, &[0; 4], 1, 1).unwrap();
            let (_, depth_view) =
                load_single_channel_texture_from_bytes(&mut factory, &[0; 4], 1, 1).unwrap();
            let (_, _, rt_color) = factory.create_render_target(1, 1).unwrap();
            let (_, _, rt_depth) = factory.create_render_target(1, 1).unwrap();
    
        Seperator {
            pso,
            pso_data: pipe::Data {
                u_view_context_l: [[0.0; 4]; 4],
                u_view_context_r: [[0.0; 4]; 4],
                u_view: [[0.0; 4]; 4],
                u_proj: [[0.0; 4]; 4],
                s_color: (color_view, sampler.clone()),
                s_depth: (depth_view, sampler),
                rt_color,
                rt_depth,
            },
        }
    }

    fn negociate_slots(&mut self, window: &Window, slots: NodeSlots) -> NodeSlots {
        let slots = slots
            .to_color_depth_input(window)
            .to_color_depth_output(window);
        let (color_view, depth_view) = slots.as_color_depth_view();
        self.pso_data.s_color = color_view;
        self.pso_data.s_depth = depth_view;
        let (color, depth) = slots.as_color_depth();
        self.pso_data.rt_color = color;
        self.pso_data.rt_depth = depth;
        slots
    }

    fn input(&mut self, head: &Head, gaze: &Gaze, flow_index: usize) -> Gaze {
        if head.view.len() >= 4 && head.proj.len() >= 4  && flow_index < 4{
            self.pso_data.u_view_context_l = head.view[0].into();
            self.pso_data.u_view_context_r = head.view[1].into();
            self.pso_data.u_view = head.view[flow_index].into();
            self.pso_data.u_proj = head.proj[flow_index].into();
        }
        gaze.clone()
    }

    fn render(&mut self, window: &Window) {
        let mut encoder = window.encoder().borrow_mut();
        encoder.draw(&gfx::Slice::from_vertex_count(12), &self.pso, &self.pso_data);
    }
}
