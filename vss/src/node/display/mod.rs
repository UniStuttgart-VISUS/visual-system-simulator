use super::*;
use wgpu::CommandEncoder;

struct Uniforms{
    resolution_in: [f32; 2],
    resolution_out: [f32; 2],

    stereo: i32,
    flow_idx: i32,
}

pub struct Display {
    pipeline: wgpu::RenderPipeline,
    uniforms: ShaderUniforms<Uniforms>,
    source_bind_group: wgpu::BindGroup,
    render_target: RenderTexture,
}

impl Display {
    pub fn new(surface: &Surface) -> Self {
        let device = surface.device().borrow_mut();
        let queue = surface.queue().borrow_mut();

        let uniforms = ShaderUniforms::new(&device, 
            Uniforms{
                stereo: 0,
                resolution_in: [1.0, 1.0],
                resolution_out: [1.0, 1.0],
                flow_idx: 0,
            }
        );
        
        let (source_bind_group_layout, source_bind_group) = placeholder_texture(&device, &queue, Some("DisplayNode s_color (placeholder)")).unwrap().create_bind_group(&device);
        let render_target = placeholder_color_rt(&device, Some("DisplayNode render_target"));

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("DisplayNode Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("mod.wgsl").into()),
        });

        let pipeline = create_render_pipeline(
            &device,
            &[&shader, &shader],
            &["vs_main", "fs_main"],
            &[&uniforms.bind_group_layout, &source_bind_group_layout],
            &[blended_color_state(COLOR_FORMAT)],
            None,
            Some("DisplayNode Render Pipeline")
        );

        Display {
            pipeline,
            uniforms,
            source_bind_group,
            render_target,
        }
    }
}

impl Node for Display {
   
    fn negociate_slots(&mut self, surface: &Surface, slots: NodeSlots, _resolution: Option<[u32;2]>, _original_image: &mut Option<Texture>) -> NodeSlots {
        let slots = slots.to_color_input(surface).to_color_output(surface, "DisplayNode");
        let device = surface.device().borrow_mut();

        self.uniforms.data.resolution_in = slots.input_size_f32();
        self.uniforms.data.resolution_out = slots.output_size_f32();

        (_, self.source_bind_group) = slots.as_color_source(&device);
        self.render_target = slots.as_color_target();

        slots
    }

    fn update_values(&mut self, _surface: &Surface, values: &ValueMap) {
        self.uniforms.data.stereo = if values
            .get("split_screen_switch")
            .unwrap_or(&Value::Bool(false))
            .as_bool()
            .unwrap_or(false)
        {
            1
        } else {
            0
        };

        self.uniforms.data.flow_idx = values.get("flow_id").unwrap_or(&Value::Number(0.0)).as_f64().unwrap_or(0.0) as i32;
    }

    fn input(&mut self, perspective: &EyePerspective, vis_param: &VisualizationParameters) -> EyePerspective {
        self.uniforms.data.flow_idx = vis_param.eye_idx as i32;

        perspective.clone()
    }

    fn render(&mut self, surface: &Surface, encoder: &mut CommandEncoder, screen: Option<&RenderTexture>) {
        self.uniforms.update(&surface.queue().borrow_mut());

        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("DisplayNode render_pass"),
            color_attachments: &[screen.unwrap_or(&self.render_target).to_color_attachment()],
            depth_stencil_attachment: None,
        });
    
        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, &self.uniforms.bind_group, &[]);
        render_pass.set_bind_group(1, &self.source_bind_group, &[]);
        if self.uniforms.data.stereo == 0 {
            render_pass.draw(0..6, 0..1);
        }else{
            render_pass.draw(0..12, 0..1);
        }
    }
}
