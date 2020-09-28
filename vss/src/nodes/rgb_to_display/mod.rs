use gfx;
use gfx::traits::FactoryExt;
use gfx::Factory;
use gfx_device_gl::CommandBuffer;
use gfx_device_gl::Resources;

use crate::pipeline::*;

gfx_defines! {
    pipeline pipe {
        u_stereo: gfx::Global<i32> = "u_stereo",
        u_resolution_in: gfx::Global<[f32; 2]> = "u_resolution_in",
        u_resolution_out: gfx::Global<[f32; 2]> = "u_resolution_out",
        s_source: gfx::TextureSampler<[f32; 4]> = "s_color",
        rt_color: gfx::RenderTarget<ColorFormat> = "rt_color",
    }
}

pub struct RgbToDisplay {
    pso: gfx::PipelineState<Resources, pipe::Meta>,
    pso_data: pipe::Data<Resources>,
}

impl Node for RgbToDisplay {
    fn new(factory: &mut gfx_device_gl::Factory) -> Self {
        let pso = factory
            .create_pipeline_simple(
                &include_glsl!("shader.vert"),
                &include_glsl!("shader.frag"),
                pipe::new(),
            )
            .unwrap();

        let sampler = factory.create_sampler_linear();
        let (_, src, dst) = factory.create_render_target(1, 1).unwrap();

        RgbToDisplay {
            pso,
            pso_data: pipe::Data {
                u_stereo: 0,
                u_resolution_in: [1.0, 1.0],
                u_resolution_out: [1.0, 1.0],
                s_source: (src, sampler),
                rt_color: dst,
            },
        }
    }

    fn update_io(
        &mut self,
        factory: &mut gfx_device_gl::Factory,
        source: Option<DeviceSource>,
        target_candidate: (Option<DeviceSource>, Option<DeviceTarget>),
    ) -> (Option<DeviceSource>, Option<DeviceTarget>) {
        let target = target_candidate.1.expect("Render target expected");
        let target_size = target.get_dimensions();

        self.pso_data.u_resolution_out = [target_size.0 as f32, target_size.1 as f32];
        self.pso_data.rt_color = target.clone();
        match source.expect("Source expected") {
            DeviceSource::Rgb {
                rgba8,
                width,
                height,
            } => {
                self.pso_data.u_resolution_in = [width as f32, height as f32];
                self.pso_data.s_source = (rgba8.clone(), factory.create_sampler_linear());
            }
            DeviceSource::RgbDepth {
                rgba8,
                width,
                height,
                ..
            } => {
                self.pso_data.u_resolution_in = [width as f32, height as f32];
                self.pso_data.s_source = (rgba8.clone(), factory.create_sampler_linear());
            }
            DeviceSource::Yuv { .. } => panic!("Unsupported source"),
        }
        (target_candidate.0, Some(target))
    }

    fn update_values(&mut self, _factory: &mut gfx_device_gl::Factory, values: &ValueMap) {
        self.pso_data.u_stereo = if values
            .get("split_screen_switch")
            .unwrap_or(&Value::Bool(false))
            .as_bool()
            .unwrap_or(false)
        {
            1
        } else {
            0
        }
    }

    fn input(&mut self, gaze: &DeviceGaze) -> DeviceGaze {
        let ratio = [
            self.pso_data.u_resolution_out[0] / self.pso_data.u_resolution_in[0],
            self.pso_data.u_resolution_out[1] / self.pso_data.u_resolution_in[1],
        ];
        let offset = [
            0.5 * (ratio[0] - ratio[1]).max(0.0),
            0.5 * (ratio[1] - ratio[0]).max(0.0),
        ];
        let scale = [
            ratio[0] / ratio[0].min(ratio[1]),
            ratio[1] / ratio[0].min(ratio[1]),
        ];

        DeviceGaze {
            x: scale[0] * gaze.x - offset[0],
            y: scale[1] * gaze.y - offset[1],
        }
    }

    fn render(&mut self, encoder: &mut gfx::Encoder<Resources, CommandBuffer>) {
        if self.pso_data.u_stereo == 0 {
            encoder.draw(&gfx::Slice::from_vertex_count(6), &self.pso, &self.pso_data);
        } else {
            encoder.draw(
                &gfx::Slice::from_vertex_count(12),
                &self.pso,
                &self.pso_data,
            );
        }
    }
}
