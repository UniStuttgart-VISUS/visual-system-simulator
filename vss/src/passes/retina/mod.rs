mod retina_map;

use self::retina_map::generate_retina_map;

use gfx;
use gfx::traits::FactoryExt;
use gfx::Factory;
use gfx_device_gl;
use gfx_device_gl::CommandBuffer;
use gfx_device_gl::Resources;

use crate::devices::*;
use crate::pipeline::*;

gfx_defines! {
    vertex Vertex {
        pos: [f32; 2] = "a_pos",
        tex: [f32; 2] = "a_tex",
    }

    pipeline pipe {
        u_resolution: gfx::Global<[f32; 2]> = "u_resolution",
        u_gaze: gfx::Global<[f32; 2]> = "u_gaze",
        s_source: gfx::TextureSampler<[f32; 4]> = "s_color",
        s_retina: gfx::TextureSampler<[f32; 4]> = "s_retina",
        rt_color: gfx::RenderTarget<ColorFormat> = "rt_color",
        vbuf: gfx::VertexBuffer<Vertex> = (),
    }
}

impl Vertex {
    fn new(p: [f32; 2], u: [f32; 2]) -> Vertex {
        Vertex { pos: p, tex: u }
    }
}

pub struct Retina {
    pso: gfx::PipelineState<Resources, pipe::Meta>,
    pso_data: pipe::Data<Resources>,
    slice: gfx::Slice<Resources>,
    vertex_buffer: gfx::handle::Buffer<Resources, Vertex>,
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

        let vertex_data = [
            Vertex::new([-1.0, -1.0], [0.0, 0.0]),
            Vertex::new([1.0, -1.0], [1.0, 0.0]),
            Vertex::new([1.0, 1.0], [1.0, 1.0]),
            Vertex::new([-1.0, -1.0], [0.0, 0.0]),
            Vertex::new([1.0, 1.0], [1.0, 1.0]),
            Vertex::new([-1.0, 1.0], [0.0, 1.0]),
        ];

        let rgba_white = vec![255; 4].into_boxed_slice();
        let (_, mask_view) = load_texture_from_bytes(factory, rgba_white, 1, 1).unwrap();

        let (vertex_buffer, slice) = factory.create_vertex_buffer_with_slice(&vertex_data, ());
        let sampler = factory.create_sampler_linear();

        let (_, src, dst) = factory.create_render_target(1, 1).unwrap();

        Retina {
            pso,
            slice,
            vertex_buffer: vertex_buffer.clone(),
            pso_data: pipe::Data {
                u_resolution: [1.0, 1.0],
                u_gaze: [0.0, 0.0],
                s_source: (src.clone(), sampler.clone()),
                s_retina: (mask_view.clone(), sampler.clone()),
                rt_color: dst.clone(),
                vbuf: vertex_buffer.clone(),
            },
        }
    }
}

impl Pass for Retina {
    fn build(&mut self, factory: &mut gfx_device_gl::Factory, vertex_data: Option<[f32; 48]>) {
        match vertex_data {
            Some(raw_data) => {
                let mut vertex_data = [Vertex::new([0.0, 0.0], [0.0, 0.0]); 12];
                for i in 0..12 {
                    vertex_data[i] = Vertex::new(
                        [raw_data[i * 4], raw_data[i * 4 + 1]],
                        [raw_data[i * 4 + 2], raw_data[i * 4 + 3]],
                    );
                }
                let (vertex_buffer, slice) =
                    factory.create_vertex_buffer_with_slice(&vertex_data, ());
                self.vertex_buffer = vertex_buffer.clone();
                self.pso_data.vbuf = vertex_buffer.clone();
                self.slice = slice;
            }
            None => {}
        }
    }

    fn update_io(
        &mut self,
        target: &DeviceTarget,
        _target_size: (u32, u32),
        source: &DeviceSource,
        source_sampler: &gfx::handle::Sampler<Resources>,
        source_size: (u32, u32),
    ) {
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
                (retinamap_view.clone(), sampler.clone())
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
        encoder.draw(&self.slice, &self.pso, &self.pso_data);
    }
}
