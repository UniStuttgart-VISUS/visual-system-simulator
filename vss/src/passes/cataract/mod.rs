use gfx;
use gfx::traits::FactoryExt;
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
        u_active: gfx::Global<i32> = "u_active",
        u_resolution: gfx::Global<[f32; 2]> = "u_resolution",
        u_blur_factor: gfx::Global<f32> = "u_blur_factor",
        u_contrast_factor: gfx::Global<f32> = "u_contrast_factor",
        s_color: gfx::TextureSampler<[f32; 4]> = "s_color",
        rt_color: gfx::RenderTarget<ColorFormat> = "rt_color",
        vbuf: gfx::VertexBuffer<Vertex> = (),
    }
}

impl Vertex {
    fn new(p: [f32; 2], u: [f32; 2]) -> Vertex {
        Vertex { pos: p, tex: u }
    }
}

pub struct Cataract {
    pso: gfx::PipelineState<Resources, pipe::Meta>,
    pso_data: pipe::Data<Resources>,
    slice: gfx::Slice<Resources>,
    vertex_buffer: gfx::handle::Buffer<Resources, Vertex>,
}

impl Cataract {
    pub fn new<F: gfx::Factory<Resources>>(factory: &mut F) -> Cataract {
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

        let (vertex_buffer, slice) = factory.create_vertex_buffer_with_slice(&vertex_data, ());
        let (_, src, dst) = factory.create_render_target(1, 1).unwrap();
        let sampler = factory.create_sampler_linear();

        Cataract {
            pso,
            slice,
            vertex_buffer: vertex_buffer.clone(),
            pso_data: pipe::Data {
                u_active: 0,
                u_resolution: [1.0, 1.0],
                u_blur_factor: 0.0,
                u_contrast_factor: 0.0,
                s_color: (src.clone(), sampler.clone()),
                rt_color: dst.clone(),
                vbuf: vertex_buffer.clone(),
            },
        }
    }
}

impl Pass for Cataract {
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
                self.pso_data.s_color = (rgba8.clone(), source_sampler.clone());
            }
            DeviceSource::RgbDepth { rgba8, d: _ } => {
                self.pso_data.s_color = (rgba8.clone(), source_sampler.clone());
            }
            DeviceSource::Yuv { .. } => panic!("Unsupported source"),
        }
        self.pso_data.u_resolution = [source_size.0 as f32, source_size.1 as f32];
    }

    fn update_params(&mut self, _factory: &mut gfx_device_gl::Factory, params: &ValueMap) {
        if let Some(Value::Bool(true)) = params.get("ct_onoff") {
            self.pso_data.u_active = 1;
            if let Some(Value::Number(ct_blur_factor)) = params.get("ct_blur_factor") {
                // ct_blur_factor is between 0 and 100
                self.pso_data.u_blur_factor = (*ct_blur_factor as f32) / 100.0;
            }
            if let Some(Value::Number(ct_contrast_factor)) = params.get("ct_contrast_factor") {
                //  ct_contrast_factor is between 0 and 100
                self.pso_data.u_contrast_factor = (*ct_contrast_factor as f32) / 100.0;
            }
        } else {
            self.pso_data.u_active = 0;
            self.pso_data.u_blur_factor = 0.0;
            self.pso_data.u_contrast_factor = 0.0;
        }
    }

    fn render(&mut self, encoder: &mut gfx::Encoder<Resources, CommandBuffer>, gaze: &DeviceGaze) {
        encoder.draw(&self.slice, &self.pso, &self.pso_data);
    }
}
