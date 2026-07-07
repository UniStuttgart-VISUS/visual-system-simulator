use crate::*;
use cgmath::Matrix4;
use cgmath::Vector3;
use std::cell::{RefCell, RefMut};
use wgpu::CommandEncoder;

/// Represents properties of eye input (perspetive and tracking).
#[derive(Clone, Debug)]
pub struct EyeInput {
    pub position: Vector3<f32>,
    pub view: Matrix4<f32>,
    pub proj: Matrix4<f32>,
    pub gaze: Vector3<f32>,
}

/// Represents properties of mouse input.
#[derive(Clone, Debug, Default)]
pub struct MouseInput {
    pub position: (f32, f32),
    pub left_button: bool,
    pub right_button: bool,
}

/// A flow encapsulates simulation nodes, i.e., all simulation and rendering.
pub struct Flow {
    nodes: RefCell<Vec<Box<dyn Node>>>,
    eye: RefCell<EyeInput>,
}

impl Flow {
    pub fn new() -> Self {
        Flow {
            nodes: RefCell::new(Vec::new()),
            eye: RefCell::new(EyeInput {
                position: Vector3::new(0.0, 0.0, 0.0),
                view: Matrix4::from_scale(1.0),
                proj: cgmath::perspective(cgmath::Deg(70.0), 1.0, 0.05, 1000.0),
                gaze: Vector3::new(0.0, 0.0, 1.0),
            }),
        }
    }

    pub fn eye_mut(&self) -> RefMut<'_, EyeInput> {
        self.eye.borrow_mut()
    }

    pub fn add_node(&mut self, node: Box<dyn Node>) {
        self.nodes.borrow_mut().push(node);
    }

    pub fn replace_node(&mut self, index: usize, node: Box<dyn Node>) {
        self.nodes.borrow_mut()[index] = node;
    }

    pub fn nodes_len(&self) -> usize {
        self.nodes.borrow().len()
    }

    pub fn negociate_slots(&self, context: &RenderContext) {
        let mut slot_a = NodeSlots::new();
        let mut slot_b = NodeSlots::new();
        let mut original_image: Option<Texture> = None;
        let nodes_len = self.nodes.borrow().len();
        for (idx, node) in self.nodes.borrow_mut().iter_mut().enumerate() {
            let suggested_slot = if idx + 1 == nodes_len {
                // Suggest window as final output.
                let device = context.device();

                let width = context.width();
                let height = context.height();

                let color_target = RenderTexture::create_color_with_format(
                    device,
                    width,
                    height,
                    context.output_format(),
                    Some("flow_negociate_slots color"),
                );

                let output_slot = Slot::Color {
                    color_source: color_target.as_texture(),
                    color_target,
                };

                NodeSlots::new_io(slot_b.take_output(), output_slot)
            } else {
                // Suggest reusing output of the pre-predecessor.
                NodeSlots::new_io(slot_b.take_output(), slot_a.take_output())
            };
            // Negociate and swap.
            slot_a = node.negociate_slots(context, suggested_slot, &mut original_image);
            std::mem::swap(&mut slot_a, &mut slot_b);
        }
    }

    pub fn inspect(&self, inspector: &dyn Inspector) -> NodeChanges {
        // Propagate to nodes.
        let mut result = NodeChanges::empty();
        for node in self.nodes.borrow_mut().iter_mut() {
            if node.inspect_config(inspector) {
                result |= node.configure();
            }
        }
        result
    }

    pub fn input(&self, mouse: &MouseInput) -> NodeChanges {
        // Propagate to nodes.
        let mut eye = self.eye.borrow().clone();
        let mut result = NodeChanges::empty();
        for node in self.nodes.borrow_mut().iter_mut().rev() {
            let (next_eye, changes) = node.input(&eye, mouse);
            eye = next_eye;
            result |= changes;
        }
        result.normalized()
    }

    pub fn render(
        &self,
        context: &RenderContext,
        encoder: &mut CommandEncoder,
        screen: &RenderTexture,
    ) {
        // Update UI if present.
        let ui_changes = self.update_ui();
        context.apply_changes(ui_changes);
        if ui_changes.contains(NodeChanges::SLOTS) {
            self.negociate_slots(context);
        }

        // Render all nodes.
        let nodes_len = self.nodes.borrow().len();
        if nodes_len == 0 {
            return;
        }
        let last_index = nodes_len - 1;
        for (idx, node) in self.nodes.borrow_mut().iter_mut().enumerate() {
            node.render(
                context,
                encoder,
                if idx == last_index {
                    Some(screen)
                } else {
                    None
                },
            );
        }
    }

    fn update_ui(&self) -> NodeChanges {
        let ui_tuple = {
            let mut nodes = self.nodes.borrow_mut();
            nodes
                .iter_mut()
                .find_map(|node| node.as_ui_mut().map(|ui_node| ui_node.begin_run()))
        };

        let Some((context, input)) = ui_tuple else {
            return NodeChanges::empty();
        };

        let mut result = NodeChanges::empty();
        let full_output = context.run_ui(input, |ctx| {
            egui::Window::new("Inspector").show(ctx, |ui| {
                egui::Grid::new("inspector_grid")
                    .num_columns(2)
                    .spacing([6.0, 4.0])
                    .striped(true)
                    .show(ui, |ui| {
                        result |= self.inspect(&UiInspector::new(ui));
                    });
            });
        });

        let mut nodes = self.nodes.borrow_mut();
        nodes
            .iter_mut()
            .find_map(|node| {
                node.as_ui_mut()
                    .map(|ui_node| ui_node.end_run(full_output.clone()))
            })
            .unwrap();

        result
    }

    pub fn post_render(&self, context: &RenderContext) {
        for node in self.nodes.borrow_mut().iter_mut() {
            node.post_render(context);
        }
    }
}
