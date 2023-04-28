use super::*;
use wgpu::CommandEncoder;

struct Uniforms {
    resolution_in: [f32; 2],
    resolution_out: [f32; 2],
}

pub struct GuiOverlay {
    pipeline: wgpu::RenderPipeline,
    uniforms: ShaderUniforms<Uniforms>,
    source_bind_group: wgpu::BindGroup,
    render_target: RenderTexture,

    gui_context: egui::Context,
    screen_descriptor: egui_wgpu::renderer::ScreenDescriptor,
    egui_input: Option<egui::RawInput>,
    egui_full_output: Option<egui::FullOutput>,
    egui_renderer: egui_wgpu::Renderer,
}

impl GuiOverlay {
    pub fn new(surface: &Surface) -> Self {
        let device = surface.device();
        let queue = surface.queue();

        let uniforms = ShaderUniforms::new(
            device,
            Uniforms {
                resolution_in: [1.0, 1.0],
                resolution_out: [1.0, 1.0],
            },
        );

        let (source_bind_group_layout, source_bind_group) =
            placeholder_texture(device, queue, Some("GuiOverlay s_color (placeholder)"))
                .unwrap()
                .create_bind_group(device);
        let render_target = placeholder_color_rt(device, Some("DisplayNode render_target"));

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("GuiOverlay Shader"),
            source: wgpu::ShaderSource::Wgsl(
                concat!(include_str!("../vert.wgsl"), include_str!("mod.wgsl")).into(),
            ),
        });

        let pipeline = create_render_pipeline(
            device,
            &[&shader, &shader],
            &["vs_main", "fs_main"],
            &[&uniforms.bind_group_layout, &source_bind_group_layout],
            &[blended_color_state(COLOR_FORMAT)],
            None,
            Some("GuiOverlay Render Pipeline"),
        );

        let gui_context = egui::Context::default();
        let screen_descriptor = egui_wgpu::renderer::ScreenDescriptor {
            size_in_pixels: [1, 1],
            pixels_per_point: 0.0,
        };
        let egui_renderer = egui_wgpu::Renderer::new(device, COLOR_FORMAT, None, 1);

        GuiOverlay {
            pipeline,
            uniforms,
            source_bind_group,
            render_target,

            gui_context,
            screen_descriptor,
            egui_input: None,
            egui_full_output: None,
            egui_renderer,
        }
    }

    pub fn begin_run(&mut self) -> (egui::Context, egui::RawInput) {
        (
            self.gui_context.clone(),
            self.egui_input.take().unwrap_or_default(),
        )
    }

    pub fn end_run(&mut self, full_output: egui::FullOutput) {
        self.egui_full_output = Some(full_output);
    }
}

impl Node for GuiOverlay {
    fn negociate_slots(
        &mut self,
        surface: &Surface,
        slots: NodeSlots,
        _original_image: &mut Option<Texture>,
    ) -> NodeSlots {
        let slots = slots
            .to_color_input(surface)
            .to_color_output(surface, "GuiOverlay");
        let device = surface.device();

        let output_size = slots.output_size_f32();
        self.uniforms.data.resolution_in = slots.input_size_f32();
        self.uniforms.data.resolution_out = output_size;

        self.screen_descriptor = egui_wgpu::renderer::ScreenDescriptor {
            size_in_pixels: [output_size[0] as u32, output_size[1] as u32],
            pixels_per_point: 1.0,
        };

        (_, self.source_bind_group) = slots.as_color_source(device);
        self.render_target = slots.as_color_target();

        slots
    }

    fn input(
        &mut self,
        eye: &EyeInput,
        mouse: &MouseInput,
    ) -> EyeInput {
        let mut egui_input = egui::RawInput::default();
        egui_input.events.push(egui::Event::PointerButton {
            pos: egui::pos2(
                mouse.position.0,
                mouse.position.1,
            ),
            button: egui::PointerButton::Primary,
            pressed: mouse.left_button,
            modifiers: egui::Modifiers::default(),
        });

        self.egui_input = Some(egui_input);

        eye.clone()
    }

    fn render(
        &mut self,
        surface: &Surface,
        encoder: &mut CommandEncoder,
        screen: Option<&RenderTexture>,
    ) {
        let full_output = self
            .egui_full_output
            .take()
            .expect("Run begin_run/end_run before render");
        let device = surface.device();
        let queue = surface.queue();

        self.uniforms.upload(queue);

        let paint_jobs = self.gui_context.tessellate(full_output.shapes);

        for texture_delta_set in full_output.textures_delta.set.iter() {
            self.egui_renderer.update_texture(
                device,
                queue,
                texture_delta_set.0,
                &texture_delta_set.1,
            );
        }
        self.egui_renderer.update_buffers(
            device,
            queue,
            encoder,
            &paint_jobs,
            &self.screen_descriptor,
        );

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

            self.egui_renderer
                .render(&mut render_pass, &paint_jobs, &self.screen_descriptor);
        }

        for texture_delta_free in full_output.textures_delta.free.iter() {
            self.egui_renderer.free_texture(texture_delta_free);
        }
    }

    fn as_ui_mut(&mut self) -> Option<&'_ mut GuiOverlay> {
        Some(self)
    }
}

pub struct UiInspector<'open> {
    ui: &'open mut egui::Ui,
}

impl<'open> UiInspector<'open> {
    pub fn new(ui: &'open mut egui::Ui) -> Self {
        Self { ui }
    }
}

impl<'open> Inspector for UiInspector<'open> {
    fn begin_flow(&mut self, index: usize) {
        self.ui.heading(format!("Flow {}", index));
        self.ui.end_row();
    }

    fn end_flow(&mut self) {}

    fn begin_node(&mut self, name: &'static str) {
        self.ui.heading(name);
        self.ui.end_row();
    }

    fn end_node(&mut self) {}

    fn mut_bool(&mut self, name: &'static str, value: &mut bool) -> bool {
        self.ui.label(name);
        let changed = self.ui.checkbox(value, "Enabled").changed();
        self.ui.end_row();
        changed
    }

    fn mut_f64(&mut self, name: &'static str, value: &mut f64) -> bool {
        self.ui.label(name);
        let changed = self.ui.add(egui::DragValue::new(value)).changed();
        self.ui.end_row();
        changed
    }

    fn mut_f32(&mut self, name: &'static str, value: &mut f32) -> bool {
        self.ui.label(name);
        let changed = self.ui.add(egui::DragValue::new(value)).changed();
        self.ui.end_row();
        changed
    }

    fn mut_i32(&mut self, name: &'static str, value: &mut i32) -> bool {
        self.ui.label(name);
        let changed = self.ui.add(egui::DragValue::new(value)).changed();
        self.ui.end_row();
        changed
    }

    fn mut_u32(&mut self, name: &'static str, value: &mut u32) -> bool {
        self.ui.label(name);
        let changed = self.ui.add(egui::DragValue::new(value)).changed();
        self.ui.end_row();
        changed
    }

    fn mut_img(&mut self, name: &'static str, value: &mut String) -> bool {
        self.ui.label(name);
        let changed = self.ui.text_edit_singleline(value).changed();
        self.ui.end_row();
        changed
    }

    fn mut_matrix(&mut self, _name: &'static str, _value: &mut Matrix4<f32>) -> bool {
        false
    }
}
