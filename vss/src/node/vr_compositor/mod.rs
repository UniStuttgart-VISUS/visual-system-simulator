use super::*;
use gfx;

gfx_defines! {
    pipeline pipe {
        u_resolution_out: gfx::Global<[f32; 2]> = "u_resolution_out",
        u_viewport: gfx::Global<[f32; 4]> = "u_viewport",
        s_source: gfx::TextureSampler<[f32; 4]> = "s_color",
        rt_color: gfx::RenderTarget<ColorFormat> = "rt_color",
    }
}

pub struct VRCompositor {
    pso: gfx::PipelineState<Resources, pipe::Meta>,
    pso_data: pipe::Data<Resources>,
}

impl VRCompositor{
    pub fn set_viewport(&mut self, x: f32, y: f32, width: f32, height: f32){
        self.pso_data.u_viewport = [x, y, width, height];
    }
}

impl Node for VRCompositor {
    fn new(surface: &Surface) -> Self {
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

        VRCompositor {
            pso,
            pso_data: pipe::Data {
                u_resolution_out: [1.0, 1.0],
                u_viewport: [0.0, 0.0, 1.0, 1.0],
                s_source: (src, sampler),
                rt_color: dst,
            },
        }
    }

    fn negociate_slots(&mut self, surface: &Surface, slots: NodeSlots) -> NodeSlots {
        let slots = slots.to_color_input(window).to_color_output(window);

        self.pso_data.u_resolution_out = slots.output_size_f32();
        self.pso_data.s_source = slots.as_color_view();
        self.pso_data.rt_color = slots.as_color();
        slots
    }

    fn render(&mut self, surface: &Surface) {
        let mut encoder = window.encoder().borrow_mut();
        encoder.draw(&gfx::Slice::from_vertex_count(6), &self.pso, &self.pso_data);
    }
}
