mod retina_map;

use gfx;

use self::retina_map::generate_retina_map;
use crate::pipeline::*;

gfx_defines! {
    pipeline pipe {
        u_resolution: gfx::Global<[f32; 2]> = "u_resolution",
        u_gaze: gfx::Global<[f32; 2]> = "u_gaze",
        s_source: gfx::TextureSampler<[f32; 4]> = "s_color",
        s_retina: gfx::TextureSampler<[f32; 4]> = "s_retina",
        rt_color: gfx::RenderTarget<ColorFormat> = "rt_color",
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
                &include_glsl!("../shader.vert"),
                &include_glsl!("shader.frag"),
                pipe::new(),
            )
            .unwrap();

        let (_, mask_view) = load_texture_from_bytes(&mut factory, &[255; 4], 1, 1).unwrap();
        let sampler = factory.create_sampler_linear();

        let (_, src, dst) = factory.create_render_target(1, 1).unwrap();

        Retina {
            pso,
            pso_data: pipe::Data {
                u_resolution: [1.0, 1.0],
                u_gaze: [0.0, 0.0],
                s_source: (src, sampler.clone()),
                s_retina: (mask_view, sampler),
                rt_color: dst,
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
        let target_size = target.get_dimensions();
        self.pso_data.u_resolution = [target_size.0 as f32, target_size.1 as f32];
        self.pso_data.rt_color = target.clone();
        match source.0.expect("Source expected") {
            DeviceSource::Rgb { rgba8, .. } => {
                self.pso_data.s_source = (rgba8.clone(), factory.create_sampler_linear());
            }
            DeviceSource::RgbDepth { rgba8, .. } => {
                self.pso_data.s_source = (rgba8.clone(), factory.create_sampler_linear());
            }
        }
        (target_candidate.0, Some(target))
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

    fn input(&mut self, _head: &Head, gaze: &DeviceGaze) -> DeviceGaze {
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
