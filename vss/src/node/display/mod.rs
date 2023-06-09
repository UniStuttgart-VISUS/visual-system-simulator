use super::*;
use wgpu::CommandEncoder;

struct Uniforms {
    viewport: [f32; 4],
    resolution_in: [f32; 2],
    resolution_out: [f32; 2],

    output_scale: u32,
    absolute_viewport: u32,
    _padding1: u32,
    _padding2: u32,
}

#[derive(Copy, Clone, Debug, Default)]
pub enum OutputScale {
    #[default]
    Fit = 0,
    Fill = 1,
    Stretch = 2,
}

impl OutputScale {
    pub fn from_string(s: &str) -> OutputScale {
        match s.to_lowercase().as_str() {
            "fit" => OutputScale::Fit,
            "fill" => OutputScale::Fill,
            "stretch" => OutputScale::Stretch,
            _ => {
                println!("Unknown OutputScale string, default value will be used");
                OutputScale::default()
            }
        }
    }
}

#[derive(Copy, Clone)]
pub struct ViewPort {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub absolute_viewport: bool,
}

pub struct Display {
    pipeline: wgpu::RenderPipeline,
    uniforms: ShaderUniforms<Uniforms>,
    source_bind_group: wgpu::BindGroup,
    render_target: RenderTexture,
}

impl Display {
    pub fn new(surface: &Surface) -> Self {
        let device = surface.device();
        let queue = surface.queue();

        let uniforms = ShaderUniforms::new(
            device,
            Uniforms {
                viewport: [0.0, 0.0, 1.0, 1.0],
                resolution_in: [1.0, 1.0],
                resolution_out: [1.0, 1.0],
                output_scale: OutputScale::default() as u32,
                absolute_viewport: 0,
                _padding1: 0,
                _padding2: 0,
            },
        );

        let (source_bind_group_layout, source_bind_group) =
            placeholder_texture(device, queue, Some("DisplayNode s_color (placeholder)"))
                .unwrap()
                .create_bind_group(device);
        let render_target = placeholder_color_rt(device, Some("DisplayNode render_target"));

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("DisplayNode Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("mod.wgsl").into()),
        });

        let pipeline = create_render_pipeline(
            device,
            &[&shader, &shader],
            &["vs_main", "fs_main"],
            &[&uniforms.bind_group_layout, &source_bind_group_layout],
            &[blended_color_state(COLOR_FORMAT)],
            None,
            Some("DisplayNode Render Pipeline"),
        );

        Display {
            pipeline,
            uniforms,
            source_bind_group,
            render_target,
        }
    }

    pub fn set_viewport(&mut self, view_port: ViewPort) {
        self.uniforms.data.viewport = [view_port.x, view_port.y, view_port.width, view_port.height];
        self.uniforms.data.absolute_viewport = view_port.absolute_viewport as u32;
    }

    pub fn set_output_scale(&mut self, output_scale: OutputScale) {
        self.uniforms.data.output_scale = output_scale as u32;
    }
}

impl Node for Display {
    fn negociate_slots(
        &mut self,
        surface: &Surface,
        slots: NodeSlots,
        _original_image: &mut Option<Texture>,
    ) -> NodeSlots {
        let slots = slots
            .to_color_input(surface)
            .to_color_output(surface, "DisplayNode");
        let device = surface.device();

        self.uniforms.data.resolution_in = slots.input_size_f32();
        self.uniforms.data.resolution_out = slots.output_size_f32();

        (_, self.source_bind_group) = slots.as_color_source(device);
        self.render_target = slots.as_color_target();

        slots
    }

    fn render(
        &mut self,
        surface: &Surface,
        encoder: &mut CommandEncoder,
        screen: Option<&RenderTexture>,
    ) {
        self.uniforms.upload(surface.queue());

        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("DisplayNode render_pass"),
            color_attachments: &[screen
                .unwrap_or(&self.render_target)
                .to_color_attachment(None)],
            depth_stencil_attachment: None,
        });

        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, &self.uniforms.bind_group, &[]);
        render_pass.set_bind_group(1, &self.source_bind_group, &[]);
        render_pass.draw(0..6, 0..1);
    }
}
