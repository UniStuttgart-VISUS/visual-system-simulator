use super::*;
use gfx;

gfx_defines! {
    pipeline pipe {
        //u_view_matrices: gfx::Global<[[[f32; 4];4];4]> = "u_view_matrices", //TODO use array of matrices if possible
        //u_proj_matrices: gfx::Global<[[[f32; 4];4];4]> = "u_proj_matrices", //TODO
        u_view_context_l: gfx::Global<[[f32; 4];4]> = "u_view_context_l",
        u_view_context_r: gfx::Global<[[f32; 4];4]> = "u_view_context_r",
        u_view_focus_l: gfx::Global<[[f32; 4];4]> = "u_view_focus_l",
        u_view_focus_r: gfx::Global<[[f32; 4];4]> = "u_view_focus_r",
        u_proj_context_l: gfx::Global<[[f32; 4];4]> = "u_proj_context_l",
        u_proj_context_r: gfx::Global<[[f32; 4];4]> = "u_proj_context_r",
        u_proj_focus_l: gfx::Global<[[f32; 4];4]> = "u_proj_focus_l",
        u_proj_focus_r: gfx::Global<[[f32; 4];4]> = "u_proj_focus_r",
        u_resolution_out: gfx::Global<[f32; 2]> = "u_resolution_out",
        s_source: gfx::TextureSampler<[f32; 4]> = "s_color",
        rt_color: gfx::RenderTarget<ColorFormat> = "rt_color",
    }
}

pub struct Varjo {
    pso: gfx::PipelineState<Resources, pipe::Meta>,
    pso_data: pipe::Data<Resources>,
}

impl Node for Varjo {
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

        Varjo {
            pso,
            pso_data: pipe::Data {
                u_view_context_l: [[0.0; 4]; 4],
                u_view_context_r: [[0.0; 4]; 4],
                u_view_focus_l: [[0.0; 4]; 4],
                u_view_focus_r: [[0.0; 4]; 4],
                u_proj_context_l: [[0.0; 4]; 4],
                u_proj_context_r: [[0.0; 4]; 4],
                u_proj_focus_l: [[0.0; 4]; 4],
                u_proj_focus_r: [[0.0; 4]; 4],
                u_resolution_out: [1.0, 1.0],
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

    fn update_values(&mut self, _window: &Window, values: &ValueMap) {
        let view_matrices = values.get("view_matrices");
        if view_matrices.is_some() {
            self.pso_data.u_view_context_l = (view_matrices.unwrap().as_matrix().unwrap()[0]).into();
            self.pso_data.u_view_context_r = (view_matrices.unwrap().as_matrix().unwrap()[1]).into();
            self.pso_data.u_view_focus_l = (view_matrices.unwrap().as_matrix().unwrap()[2]).into();
            self.pso_data.u_view_focus_r = (view_matrices.unwrap().as_matrix().unwrap()[3]).into();
        }
        let proj_matrices = values.get("proj_matrices");
        if proj_matrices.is_some() {
            self.pso_data.u_proj_context_l = (proj_matrices.unwrap().as_matrix().unwrap()[0]).into();
            self.pso_data.u_proj_context_r = (proj_matrices.unwrap().as_matrix().unwrap()[1]).into();
            self.pso_data.u_proj_focus_l = (proj_matrices.unwrap().as_matrix().unwrap()[2]).into();
            self.pso_data.u_proj_focus_r = (proj_matrices.unwrap().as_matrix().unwrap()[3]).into();
        }
    }

    fn render(&mut self, window: &Window) {
        let mut encoder = window.encoder().borrow_mut();
        encoder.draw(&gfx::Slice::from_vertex_count(24), &self.pso, &self.pso_data);
    }
}
