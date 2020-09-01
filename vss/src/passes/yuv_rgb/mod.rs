use gfx;
use gfx::traits::FactoryExt;
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
        s_y: gfx::TextureSampler<f32> = "s_y",
        s_u: gfx::TextureSampler<f32> = "s_u",
        s_v: gfx::TextureSampler<f32> = "s_v",
        rt_color: gfx::RenderTarget<ColorFormat> = "rt_color",
        vbuf: gfx::VertexBuffer<Vertex> = (),
    }
}

impl Vertex {
    fn new(p: [f32; 2], u: [f32; 2]) -> Vertex {
        Vertex { pos: p, tex: u }
    }
}

pub struct YuvRgb {
    pso: gfx::PipelineState<Resources, pipe::Meta>,
    pso_data: pipe::Data<Resources>,
    slice: gfx::Slice<Resources>,
    vertex_buffer: gfx::handle::Buffer<Resources, Vertex>,
}

impl YuvRgb {
    pub fn new<F: gfx::Factory<Resources>>(factory: &mut F) -> Self {
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
        let sampler = factory.create_sampler_linear();
        let (_, srv, _): (
            _,
            _,
            gfx::handle::RenderTargetView<gfx_device_gl::Resources, f32>,
        ) = factory.create_render_target(1, 1).unwrap();
        let (_, _, rtv) = factory.create_render_target(1, 1).unwrap();

        YuvRgb {
            pso,
            slice,
            vertex_buffer: vertex_buffer.clone(),
            pso_data: pipe::Data {
                s_y: (srv.clone(), sampler.clone()),
                s_u: (srv.clone(), sampler.clone()),
                s_v: (srv, sampler),
                rt_color: rtv,
                vbuf: vertex_buffer,
            },
        }
    }
}

impl Pass for YuvRgb {
    fn build(&mut self, factory: &mut gfx_device_gl::Factory, vertex_data: Option<[f32; 48]>) {
        if let Some(raw_data) = vertex_data {
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
            self.pso_data.vbuf = vertex_buffer;
            self.slice = slice;
        }
    }

    fn update_io(
        &mut self,
        target: &DeviceTarget,
        _target_size: (u32, u32),
        source: &DeviceSource,
        source_sampler: &gfx::handle::Sampler<Resources>,
        _source_size: (u32, u32),
    ) {
        self.pso_data.rt_color = target.clone();
        match source {
            DeviceSource::Rgb { .. } => panic!("Unsupported source"),
            DeviceSource::RgbDepth { .. } => panic!("Unsupported source"),
            DeviceSource::Yuv { y, u, v } => {
                self.pso_data.s_y = (y.clone(), source_sampler.clone());
                self.pso_data.s_u = (u.clone(), source_sampler.clone());
                self.pso_data.s_v = (v.clone(), source_sampler.clone());
            }
        }
    }

    fn update_params(&mut self, _factory: &mut gfx_device_gl::Factory, _values: &ValueMap) {}

    fn render(&mut self, encoder: &mut gfx::Encoder<Resources, CommandBuffer>, _gaze: &DeviceGaze) {
        encoder.draw(&self.slice, &self.pso, &self.pso_data);
    }
}
