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

    pub fn eye_mut(&self) -> RefMut<EyeInput> {
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

    pub fn negociate_slots(&self, surface: &Surface) {
        let mut slot_a = NodeSlots::new();
        let mut slot_b = NodeSlots::new();
        let mut original_image: Option<Texture> = None;
        let nodes_len = self.nodes.borrow().len();
        for (idx, node) in self.nodes.borrow_mut().iter_mut().enumerate() {
            let suggested_slot = if idx + 1 == nodes_len {
                // Suggest window as final output.
                let device = surface.device();

                let width = surface.width();
                let height = surface.height();

                let color_target =
                    create_color_rt(device, width, height, Some("flow_negociate_slots color"));
                let deflection_target = create_highp_rt(
                    device,
                    width,
                    height,
                    Some("flow_negociate_slots deflection"),
                );
                let color_change_target = create_highp_rt(
                    device,
                    width,
                    height,
                    Some("flow_negociate_slots color_change"),
                );
                let color_uncertainty_target = create_highp_rt(
                    device,
                    width,
                    height,
                    Some("flow_negociate_slots color_uncertainty"),
                );
                let covariances_target = create_highp_rt(
                    device,
                    width,
                    height,
                    Some("flow_negociate_slots covariances"),
                );

                let output_slot = Slot::Rgb {
                    color_source: color_target.as_texture(),
                    color_target,
                    deflection_source: deflection_target.as_texture(),
                    deflection_target,
                    color_change_source: color_change_target.as_texture(),
                    color_change_target,
                    color_uncertainty_source: color_uncertainty_target.as_texture(),
                    color_uncertainty_target,
                    covariances_source: covariances_target.as_texture(),
                    covariances_target,
                };

                NodeSlots::new_io(slot_b.take_output(), output_slot)
            } else {
                // Suggest reusing output of the pre-predecessor.
                NodeSlots::new_io(slot_b.take_output(), slot_a.take_output())
            };
            // Negociate and swap.
            slot_a = node.negociate_slots(surface, suggested_slot, &mut original_image);
            std::mem::swap(&mut slot_a, &mut slot_b);
        }
    }

    pub fn inspect(&self, inspector: &mut dyn Inspector) {
        // Propagate to nodes.
        for node in self.nodes.borrow_mut().iter_mut() {
            inspector.mut_node(node);
        }
    }

    pub fn input(&self, mouse: &MouseInput) {
        // Propagate to nodes.
        let mut eye = self.eye.borrow().clone();
        for node in self.nodes.borrow_mut().iter_mut().rev() {
            eye = node.input(&eye, mouse);
        }
    }

    pub fn render(&self, surface: &Surface, encoder: &mut CommandEncoder, screen: &RenderTexture) {
        // Update UI if present.
        self.update_ui();

        // Render all nodes.
        let last_index = self.nodes.borrow_mut().len() - 1;
        for (idx, node) in self.nodes.borrow_mut().iter_mut().enumerate() {
            node.render(
                surface,
                encoder,
                if idx == last_index {
                    Some(screen)
                } else {
                    None
                },
            );
        }
    }

    fn update_ui(&self) {
        let ui_tuple = self
            .nodes
            .borrow_mut()
            .iter_mut()
            .find_map(|node| node.as_ui_mut().map(|ui_node| ui_node.begin_run()));

        if let Some((context, input)) = ui_tuple {
            let full_output = context.run(input, |ctx| {
                egui::Window::new("Inspector").show(ctx, |ui| {
                    egui::Grid::new("inspector_grid")
                        .num_columns(2)
                        .spacing([6.0, 4.0])
                        .striped(true)
                        .show(ui, |ui| {
                            self.inspect(&mut UiInspector::new(ui));
                        });
                });
            });

            self.nodes
                .borrow_mut()
                .iter_mut()
                .find_map(|node| {
                    node.as_ui_mut()
                        .map(|ui_node| ui_node.end_run(full_output.clone()))
                })
                .unwrap();
        }
    }

    pub fn post_render(&self, surface: &Surface) {
        for node in self.nodes.borrow_mut().iter_mut() {
            node.post_render(surface);
        }
    }
}
