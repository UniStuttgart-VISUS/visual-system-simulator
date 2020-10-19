use super::*;
use gfx;
use std::f32;
use std::io::Cursor;

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
    }
}

pub struct Lens {
    pso: gfx::PipelineState<Resources, pipe::Meta>,
    pso_data: pipe::Data<Resources>,
}

impl Node for Lens {
    fn new(window: &Window) -> Self {
        let mut factory = window.factory().borrow_mut();
        let pso = factory
            .create_pipeline_simple(
                &include_glsl!("../mod.vert"),
                &include_glsl!("mod.frag"),
                pipe::new(),
            )
            .unwrap();

        //TODO: this is one stupid and slow hack!!! pre-compute this properly
        let filename_normal = Cursor::new(include_bytes!("normal.png").to_vec());
        let (_, normal_view) = load_highres_normalmap(&mut factory, filename_normal).unwrap();

        let (_, cornea_view) = load_texture_from_bytes(&mut factory, &[127; 4], 1, 1).unwrap();

        let sampler = factory.create_sampler_linear();
        let (_, src, dst) = factory.create_render_target(1, 1).unwrap();

        let (_, srv, _): (
            _,
            _,
            gfx::handle::RenderTargetView<gfx_device_gl::Resources, f32>,
        ) = factory.create_render_target(1, 1).unwrap();

        Lens {
            pso,
            pso_data: pipe::Data {
                u_active: 0,
                u_samplecount: 4,
                u_depth_min: 200.0,  //XXX: was 1000.0 - 300.0,
                u_depth_max: 5000.0, //XXX: was 1000.0 + 0.0,
                u_near_point: 0.0,
                u_far_point: f32::INFINITY,
                u_near_vision_factor: 0.0,
                u_far_vision_factor: 0.0,
                s_color: (src, sampler.clone()),
                s_depth: (srv, sampler.clone()),
                s_normal: (normal_view, sampler.clone()),
                s_cornea: (cornea_view, sampler),
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
        self.pso_data.rt_color = target.clone();
        match source.0.expect("Source expected") {
            NodeSource::Rgb { color, .. } => {
                self.pso_data.s_color = (color.clone(), factory.create_sampler_linear());
            }
            NodeSource::RgbDepth { color, depth, .. } => {
                self.pso_data.s_color = (color.clone(), factory.create_sampler_linear());
                self.pso_data.s_depth = (depth.clone(), factory.create_sampler_linear());
            }
        }
        (target_candidate.0, Some(target))
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
}
