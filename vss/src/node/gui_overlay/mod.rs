use super::*;
use eframe::epaint::Primitive;
use wgpu::CommandEncoder;

use egui_wgpu;

struct Uniforms{
    resolution_in: [f32; 2],
    resolution_out: [f32; 2],
}

pub struct GuiOverlay {
    pipeline: wgpu::RenderPipeline,
    uniforms: ShaderUniforms<Uniforms>,
    source_bind_group: wgpu::BindGroup,
    render_target: RenderTexture,

    gui_context: eframe::egui::Context,
    screen_descriptor: egui_wgpu::renderer::ScreenDescriptor,
    egui_input: Option<eframe::egui::RawInput>,
    egui_renderer: egui_wgpu::Renderer,
}

impl GuiOverlay {
    pub fn new(surface: &Surface) -> Self {
        let device = surface.device().borrow_mut();
        let queue = surface.queue().borrow_mut();

        let uniforms = ShaderUniforms::new(
            &device,
            Uniforms {
                resolution_in: [1.0, 1.0],
                resolution_out: [1.0, 1.0],
            },
        );
        
        let (source_bind_group_layout, source_bind_group) =
            placeholder_texture(&device, &queue, Some("GuiOverlay s_color (placeholder)"))
                .unwrap()
                .create_bind_group(&device);
        let render_target = placeholder_color_rt(&device, Some("DisplayNode render_target"));

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("GuiOverlay Shader"),
            source: wgpu::ShaderSource::Wgsl(
                concat!(
                    include_str!("../vert.wgsl"),
                    include_str!("mod.wgsl")
                )
                .into()
            ),
        });

        let pipeline = create_render_pipeline(
            &device,
            &[&shader, &shader],
            &["vs_main", "fs_main"],
            &[&uniforms.bind_group_layout, &source_bind_group_layout],
            &[blended_color_state(COLOR_FORMAT)],
            None,
            Some("GuiOverlay Render Pipeline")
        );
        
        let gui_context = eframe::egui::Context::default();
        let screen_descriptor = egui_wgpu::renderer::ScreenDescriptor{
            size_in_pixels: [1, 1],
            pixels_per_point: 0.0,
        };
        let egui_renderer = egui_wgpu::Renderer::new(&device, COLOR_FORMAT, None, 1);

        GuiOverlay {
            pipeline,
            uniforms,
            source_bind_group,
            render_target,
            
            gui_context,
            screen_descriptor,
            egui_input: None,
            egui_renderer,
        }
    }
}

impl Node for GuiOverlay {
   
    fn negociate_slots(
        &mut self,
        surface: &Surface,
        slots: NodeSlots,
        _original_image: &mut Option<Texture>,
    ) -> NodeSlots {
        let slots = slots.to_color_input(surface).to_color_output(surface, "GuiOverlay");
        let device = surface.device().borrow_mut();

        let output_size = slots.output_size_f32();
        self.uniforms.data.resolution_in = slots.input_size_f32();
        self.uniforms.data.resolution_out = output_size;

        self.screen_descriptor = egui_wgpu::renderer::ScreenDescriptor{
            size_in_pixels: [output_size[0] as u32, output_size[1] as u32],
            pixels_per_point: 0.0,
        };

        (_, self.source_bind_group) = slots.as_color_source(&device);
        self.render_target = slots.as_color_target();

        slots
    }

    fn input(
        &mut self,
        perspective: &EyePerspective,
        vis_param: &VisualizationParameters,
    ) -> EyePerspective {
        let mut egui_input = eframe::egui::RawInput::default();
        egui_input.events.push(eframe::egui::Event::PointerButton{
            pos: eframe::egui::pos2(vis_param.mouse_input.position.0, vis_param.mouse_input.position.1),
            button: eframe::egui::PointerButton::Primary,
            pressed: vis_param.mouse_input.left_button,
            modifiers: eframe::egui::Modifiers::default(),
        });

        self.egui_input = Some(egui_input);

        perspective.clone()
    }

    fn render(
        &mut self,
        surface: &Surface,
        encoder: &mut CommandEncoder,
        screen: Option<&RenderTexture>,
    ) {
        let device = surface.device().borrow_mut();
        let queue = surface.queue().borrow_mut();

        self.uniforms.update(&queue);

        self.gui_context.begin_frame(self.egui_input.take().unwrap_or_default());

        eframe::egui::Window::new("Window").show(&self.gui_context, |ui| {
            if ui.button("Click me").clicked() {
                println!("Click");
            }
        });

        let full_output = self.gui_context.end_frame();
        let paint_jobs = self.gui_context.tessellate(full_output.shapes);
        for texture_delta_set in full_output.textures_delta.set.iter(){
            self.egui_renderer.update_texture(&device, &queue, texture_delta_set.0, &texture_delta_set.1);
        };
        self.egui_renderer.update_buffers(&device, &queue, encoder, &paint_jobs, &self.screen_descriptor);

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("GuiOverlay Main render_pass"),
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

        {//TODO: this render pass could be potentially combined with the previous one
            let mut egui_render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("GuiOverlay GUI render_pass"),
                color_attachments: &[screen
                    .unwrap_or(&self.render_target)
                    .to_color_attachment(None)],
                depth_stencil_attachment: None,
            });
    
            self.egui_renderer.render(&mut egui_render_pass, &paint_jobs, &self.screen_descriptor);
        }

        for texture_delta_free in full_output.textures_delta.free.iter(){
            self.egui_renderer.free_texture(texture_delta_free);
        };

    }
}
