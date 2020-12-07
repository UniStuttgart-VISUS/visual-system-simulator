mod retina_map;

use self::retina_map::generate_retina_map;
use super::*;
use gfx;
use gfx::format::Rgba32F;


gfx_defines! {
    pipeline pipe {
        u_resolution: gfx::Global<[f32; 2]> = "u_resolution",
        u_gaze: gfx::Global<[f32; 2]> = "u_gaze",
        s_source: gfx::TextureSampler<[f32; 4]> = "s_color",
        s_retina: gfx::TextureSampler<[f32; 4]> = "s_retina",
        rt_color: gfx::RenderTarget<ColorFormat> = "rt_color",
        s_deflection: gfx::TextureSampler<[f32; 4]> = "s_deflection",
        rt_deflection: gfx::RenderTarget<Rgba32F> = "rt_deflection",
        s_color_change: gfx::TextureSampler<[f32; 4]> = "s_color_change",
        rt_color_change: gfx::RenderTarget<Rgba32F> = "rt_color_change",
        s_color_uncertainty: gfx::TextureSampler<[f32; 4]> = "s_color_uncertainty",
        rt_color_uncertainty: gfx::RenderTarget<Rgba32F> = "rt_color_uncertainty",

    }
}

pub struct Retina {
    pso: gfx::PipelineState<Resources, pipe::Meta>,
    pso_data: pipe::Data<Resources>,
}

impl Node for Retina {
    fn new(window: &Window) -> Self {
        let mut factory = window.factory().borrow_mut();
        let pso = factory
            .create_pipeline_simple(
                &include_glsl!("../mod.vert"),
                &include_glsl!("mod.frag"),
                pipe::new(),
            )
            .unwrap();

        let (_, mask_view) = load_texture_from_bytes(&mut factory, &[255; 4], 1, 1).unwrap();
        let sampler = factory.create_sampler_linear();

        let (_, src, dst) = factory.create_render_target(1, 1).unwrap();
        let (_, s_deflection, rt_deflection) = factory.create_render_target(1, 1).unwrap();
        let (_, s_color_change, rt_color_change) = factory.create_render_target(1, 1).unwrap();
        let (_, s_color_uncertainty, rt_color_uncertainty) = factory.create_render_target(1, 1).unwrap();

        Retina {
            pso,
            pso_data: pipe::Data {
                u_resolution: [1.0, 1.0],
                u_gaze: [0.0, 0.0],
                s_source: (src, sampler.clone()),
                s_retina: (mask_view, sampler.clone()),
                rt_color: dst,
                s_deflection:(s_deflection, sampler.clone()),
                rt_deflection,
                s_color_change:(s_color_change, sampler.clone()),
                rt_color_change,
                s_color_uncertainty:(s_color_uncertainty, sampler.clone()),
                rt_color_uncertainty
            },
        }
    }

    fn negociate_slots(&mut self, window: &Window, slots: NodeSlots) -> NodeSlots {
        let slots = slots.to_color_input(window).to_color_output(window);
        self.pso_data.u_resolution = slots.output_size_f32();
        self.pso_data.s_source = slots.as_color_view();
        self.pso_data.rt_color = slots.as_color();
        self.pso_data.s_deflection = slots.as_deflection_view();
        self.pso_data.rt_deflection = slots.as_deflection();
        self.pso_data.s_color_change = slots.as_color_change_view();
        self.pso_data.rt_color_change = slots.as_color_change();  
        self.pso_data.s_color_uncertainty = slots.as_color_uncertainty_view();
        self.pso_data.rt_color_uncertainty = slots.as_color_uncertainty();

        slots
    }

    fn update_values(&mut self, window: &Window, values: &ValueMap) {
        let mut factory = window.factory().borrow_mut();
        if let Some(Value::Image(retina_map_path)) = values.get("retina_map_path") {
            let (_, retinamap_view) = load_texture(&mut factory, load(retina_map_path)).unwrap();
            let sampler = self.pso_data.s_retina.clone().1;
            //XXX: check resolution!

            self.pso_data.s_retina = (retinamap_view, sampler);
        } else {
            let target_resolution = (
                self.pso_data.u_resolution[0] as u32,
                self.pso_data.u_resolution[1] as u32,
            );
            let retina_map = generate_retina_map(target_resolution, &values);
            let (_, retinamap_view) = load_texture_from_bytes(
                &mut factory,
                &retina_map,
                target_resolution.0,
                target_resolution.1,
            )
            .unwrap();

            self.pso_data.s_retina = (retinamap_view, self.pso_data.s_retina.clone().1);
        };
    }

    fn input(&mut self, _head: &Head, gaze: &Gaze, _vis_param: &VisualizationParameters, _flow_index: usize) -> Gaze {
        self.pso_data.u_gaze = [
            gaze.x * self.pso_data.u_resolution[0],
            gaze.y * self.pso_data.u_resolution[1],
        ];
        gaze.clone()
    }

    fn render(&mut self, window: &Window) {
        let mut encoder = window.encoder().borrow_mut();
        encoder.draw(&gfx::Slice::from_vertex_count(6), &self.pso, &self.pso_data);
    }
}
