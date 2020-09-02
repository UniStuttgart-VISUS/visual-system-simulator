mod retina_map;

use self::retina_map::generate_retina_map;

use gfx;
use gfx::traits::FactoryExt;
use gfx::Factory;
use gfx_device_gl::CommandBuffer;
use gfx_device_gl::Resources;

use crate::devices::*;
use crate::pipeline::*;

gfx_defines! {
    pipeline pipe {
        u_stereo: gfx::Global<i32> = "u_stereo",
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

impl Retina {
    pub fn new(factory: &mut gfx_device_gl::Factory) -> Retina {
        let pso = factory
            .create_pipeline_simple(
                &include_glsl!("../shader.vert"),
                &include_glsl!("shader.frag"),
                pipe::new(),
            )
            .unwrap();

        let rgba_white = vec![255; 4].into_boxed_slice();
        let (_, mask_view) = load_texture_from_bytes(factory, rgba_white, 1, 1).unwrap();
        let sampler = factory.create_sampler_linear();

        let (_, src, dst) = factory.create_render_target(1, 1).unwrap();

        Retina {
            pso,
            pso_data: pipe::Data {
                u_stereo: 0,
                u_resolution: [1.0, 1.0],
                u_gaze: [0.0, 0.0],
                s_source: (src, sampler.clone()),
                s_retina: (mask_view, sampler),
                rt_color: dst,
            },
        }
    }
}

impl Pass for Retina {
    fn update_io(
        &mut self,
        target: &DeviceTarget,
        _target_size: (u32, u32),
        source: &DeviceSource,
        source_sampler: &gfx::handle::Sampler<Resources>,
        source_size: (u32, u32),
        stereo: bool,
    ) {
        self.pso_data.u_stereo = if stereo { 1 } else { 0 };
        self.pso_data.rt_color = target.clone();
        match source {
            DeviceSource::Rgb { rgba8 } => {
                self.pso_data.s_source = (rgba8.clone(), source_sampler.clone());
            }
            DeviceSource::RgbDepth { rgba8, d: _ } => {
                self.pso_data.s_source = (rgba8.clone(), source_sampler.clone());
            }
            DeviceSource::Yuv { .. } => panic!("Unsupported source"),
        }
        self.pso_data.u_resolution = [source_size.0 as f32, source_size.1 as f32];
    }

    fn update_params(&mut self, factory: &mut gfx_device_gl::Factory, params: &ValueMap) {
        // update retina map
        self.pso_data.s_retina =
            if let Some(Value::Image(retina_map_path)) = params.get("retina_map_path") {
                println!("[retina] using map {:?}", retina_map_path);
                let (_, retinamap_view) = load_texture(factory, load(retina_map_path)).unwrap();
                let sampler = self.pso_data.s_retina.clone().1;
                (retinamap_view, sampler)
            } else {
                println!("[retina] generating map");
                let retina_resolution = (
                    self.pso_data.u_resolution[0] as u32,
                    self.pso_data.u_resolution[1] as u32,
                );
                let retina_map = generate_retina_map(retina_resolution, &params);
                let (_, retinamap_view) = load_texture_from_bytes(
                    factory,
                    retina_map,
                    retina_resolution.0,
                    retina_resolution.1,
                )
                .unwrap();
                (retinamap_view, self.pso_data.s_retina.clone().1)
            };
    }

    fn render(&mut self, encoder: &mut gfx::Encoder<Resources, CommandBuffer>, gaze: &DeviceGaze) {
        self.pso_data.u_gaze = [gaze.x, gaze.y];
        encoder.draw(&gfx::Slice::from_vertex_count(6), &self.pso, &self.pso_data);
    }
}
