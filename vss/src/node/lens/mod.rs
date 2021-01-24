mod generator;

pub use generator::*;

use super::*;
use gfx;
use std::f32;
use gfx::format::Rgba32F;


const DIOPTRES_SCALING: f32 = 0.332_763_369_417_523 as f32;

gfx_defines! {
    pipeline pipe {
        u_active: gfx::Global<i32> = "u_active",
        u_samplecount: gfx::Global<i32> = "u_samplecount",
        u_depth_min: gfx::Global<f32> = "u_depth_min",
        u_depth_max: gfx::Global<f32> = "u_depth_max",
        // smallest distance on which the eye can focus, in mm
        u_near_point: gfx::Global<f32> = "u_near_point",
        // largest  distance on which the eye can focus, in mm
        u_far_point: gfx::Global<f32> = "u_far_point",
        // determines the bluriness of objects that are too close to focus
        // should be between 0 and 2
        u_near_vision_factor: gfx::Global<f32> = "u_near_vision_factor",
        // determines the bluriness of objects that are too far to focus
        // should be between 0 and 2
        u_far_vision_factor: gfx::Global<f32> = "u_far_vision_factor",
        s_color: gfx::TextureSampler<[f32; 4]> = "s_color",
        s_depth: gfx::TextureSampler<f32> = "s_depth",
        s_normal: gfx::TextureSampler<[f32; 4]> = "s_normal",
        s_cornea: gfx::TextureSampler<[f32; 4]> = "s_cornea",
        rt_color: gfx::RenderTarget<ColorFormat> = "rt_color",
        s_deflection: gfx::TextureSampler<[f32; 4]> = "s_deflection",
        rt_deflection: gfx::RenderTarget<Rgba32F> = "rt_deflection",
        s_color_change: gfx::TextureSampler<[f32; 4]> = "s_color_change",
        rt_color_change: gfx::RenderTarget<Rgba32F> = "rt_color_change",
        s_color_uncertainty: gfx::TextureSampler<[f32; 4]> = "s_color_uncertainty",
        rt_color_uncertainty: gfx::RenderTarget<Rgba32F> = "rt_color_uncertainty",

        u_dir_calc_scale: gfx::Global<f32> = "u_dir_calc_scale",
    }
}

pub struct Lens {
    generator: NormalMapGenerator,
    pso: gfx::PipelineState<Resources, pipe::Meta>,
    pso_data: pipe::Data<Resources>,
}

impl Node for Lens {
    fn new(window: &Window) -> Self {
        let generator = NormalMapGenerator::new(&window);
        let mut factory = window.factory().borrow_mut();
        let pso = factory
            .create_pipeline_simple(
                &include_glsl!("../mod.vert"),
                &include_glsl!("mod.frag"),
                pipe::new(),
            )
            .unwrap();

        //TODO: use generalized load_texture_from_bytes
        let (_, normal_view) = load_highp_texture_from_bytes(&mut factory, &[127; 4], 1, 1).unwrap();

        let (_, cornea_view) = load_texture_from_bytes(&mut factory, &[127; 4], 1, 1).unwrap();

        let sampler = factory.create_sampler_linear();
        let (_, src, dst) = factory.create_render_target(1, 1).unwrap();

        let (_, srv, _): (
            _,
            _,
            gfx::handle::RenderTargetView<gfx_device_gl::Resources, f32>,
        ) = factory.create_render_target(1, 1).unwrap();
        let (_, s_deflection, rt_deflection) = factory.create_render_target(1, 1).unwrap();
        let (_, s_color_change, rt_color_change) = factory.create_render_target(1, 1).unwrap();
        let (_, s_color_uncertainty, rt_color_uncertainty) = factory.create_render_target(1, 1).unwrap();


        Lens {
            generator,
            pso,
            pso_data: pipe::Data {
                u_active: 0,
                u_samplecount: 4,
                //u_depth_min: 200.0,  //XXX: was 1000.0 - 300.0,
                u_depth_min: 100.0,  //XXX: was 1000.0 - 300.0,
                //u_depth_max: 5000.0, //XXX: was 1000.0 + 0.0,
                u_depth_max: 1800.0, //XXX: was 1000.0 + 0.0,
                u_near_point: 0.0,
                u_far_point: f32::INFINITY,
                u_near_vision_factor: 0.0,
                u_far_vision_factor: 0.0,
                s_color: (src, sampler.clone()),
                s_depth: (srv, sampler.clone()),
                s_normal: (normal_view, sampler.clone()),
                s_cornea: (cornea_view, sampler.clone()),
                rt_color: dst,
                s_deflection:(s_deflection, sampler.clone()),
                rt_deflection,
                s_color_change:(s_color_change, sampler.clone()),
                rt_color_change,
                s_color_uncertainty:(s_color_uncertainty, sampler.clone()),
                rt_color_uncertainty,
                u_dir_calc_scale: 1.0
            },
        }
    }

    fn negociate_slots(&mut self, window: &Window, slots: NodeSlots) -> NodeSlots {
        use gfx::format;

        let slots = slots.to_color_depth_input(window).to_color_output(window);
        let (color_view, depth_view) = slots.as_color_depth_view();
        self.pso_data.s_color = color_view;
        self.pso_data.s_depth = depth_view;

        let size = slots.output_size_f32();
        self.generator.generate(window, size[0] as u16, size[1] as u16);
        let mut factory = window.factory().borrow_mut();
        let normal_texture = factory
        .view_texture_as_shader_resource::<(gfx::format::R32_G32_B32_A32, gfx::format::Float)>(
            &self.generator.texture,
            (0, 0),
            format::Swizzle::new(),
        )
        .unwrap();
        self.pso_data.s_normal = (normal_texture, factory.create_sampler_linear());

        self.pso_data.rt_color = slots.as_color();
        self.pso_data.s_deflection = slots.as_deflection_view();
        self.pso_data.rt_deflection = slots.as_deflection();
        self.pso_data.s_color_change = slots.as_color_change_view();
        self.pso_data.rt_color_change = slots.as_color_change();  
        self.pso_data.s_color_uncertainty = slots.as_color_uncertainty_view();
        self.pso_data.rt_color_uncertainty = slots.as_color_uncertainty();
        slots
    }

    fn update_values(&mut self, _window: &Window, values: &ValueMap) {
        // default values
        self.pso_data.u_near_point = 0.0;
        self.pso_data.u_far_point = f32::INFINITY;
        self.pso_data.u_near_vision_factor = 0.0;
        self.pso_data.u_far_vision_factor = 0.0;
        self.pso_data.u_active = 0;

        if let Some(Value::Bool(true)) = values.get("presbyopia_onoff") {
            // near point is a parameter between 0 and 100 that is to be scaled to 0 - 1000
            if let Some(Value::Number(near_point)) = values.get("presbyopia_near_point") {
                self.pso_data.u_active = 1;
                self.pso_data.u_near_point = (near_point * 10.0) as f32;
                self.pso_data.u_near_vision_factor = 1.0;
            }
        }

        if let Some(Value::Bool(true)) = values.get("myopiahyperopia_onoff") {
            if let Some(Value::Number(mnh)) = values.get("myopiahyperopia_mnh") {
                self.pso_data.u_active = 1;
                // mnh represents a range of -3D to 3D
                let dioptres = ((mnh / 50.0 - 1.0) * 3.0) as f32;

                if dioptres < 0.0 {
                    // myopia
                    self.pso_data.u_far_point = -1000.0 / dioptres;
                    // u_near_point should not be farther than u_far_point
                    self.pso_data.u_near_point =
                        self.pso_data.u_near_point.min(self.pso_data.u_far_point);
                    let vision_factor = 1.0 - dioptres * DIOPTRES_SCALING;
                    self.pso_data.u_far_vision_factor =
                        self.pso_data.u_far_vision_factor.max(vision_factor as f32);
                } else if dioptres > 0.0 {
                    // hyperopia
                    let hyperopia_near_point = 1000.0 / (4.4 - dioptres);
                    self.pso_data.u_near_point =
                        self.pso_data.u_near_point.max(hyperopia_near_point);
                    let vision_factor = 1.0 + dioptres * DIOPTRES_SCALING;
                    self.pso_data.u_near_vision_factor =
                        self.pso_data.u_near_vision_factor.max(vision_factor as f32);
                }
            }
        }
    }

    fn render(&mut self, window: &Window) {
        let mut encoder = window.encoder().borrow_mut();
        encoder.draw(&gfx::Slice::from_vertex_count(6), &self.pso, &self.pso_data);
    }

    fn input(&mut self, perspective: &EyePerspective, vis_param: &VisualizationParameters) -> EyePerspective {
        self.pso_data.u_dir_calc_scale = vis_param.dir_calc_scale;
        self.pso_data.u_depth_max = vis_param.test_depth_max;
        self.pso_data.u_depth_min = vis_param.test_depth_min;
        perspective.clone()
    }
}
