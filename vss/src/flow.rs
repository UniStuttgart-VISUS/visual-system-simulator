use crate::*;
use cgmath::Matrix4;
use cgmath::Vector3;
use std::cell::{RefCell, RefMut};
use wgpu::CommandEncoder;

/// Represents properties of eye-tracking data.
#[derive(Debug, Clone)]
pub struct EyePerspective {
    pub position: Vector3<f32>,
    pub view: Matrix4<f32>,
    pub proj: Matrix4<f32>,
    pub gaze: Vector3<f32>,
}

/// A flow encapsulates simulation nodes, i.e., all simulation and rendering.
pub struct Flow {
    nodes: RefCell<Vec<Box<dyn Node>>>,
    last_slot: RefCell<Option<NodeSlots>>,
    perspective: RefCell<EyePerspective>,
}

impl Flow {
    pub fn new() -> Self {
        Flow {
            nodes: RefCell::new(Vec::new()),
            last_slot: RefCell::new(None),
            perspective: RefCell::new(EyePerspective {
                position: Vector3::new(0.0, 0.0, 0.0),
                view: Matrix4::from_scale(1.0),
                proj: cgmath::perspective(cgmath::Deg(70.0), 1.0, 0.05, 1000.0),
                gaze: Vector3::new(0.0, 0.0, 1.0),
            }),
        }
    }

    pub fn perspective_mut(&self) -> RefMut<EyePerspective> {
        self.perspective.borrow_mut()
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

    // pub fn update_last_slot(&self, surface: &Surface) {
    //     if self.last_slot.borrow().is_some() {
    //         let output = self.last_slot.borrow_mut().as_mut().unwrap().take_output();
    //         let suggested_slot =
    //             NodeSlots::new_io(
    //                 window,
    //                 self.last_slot.borrow_mut().as_mut().unwrap().take_input(),
    //                 match output {
    //                     Slot::Rgb{
    //                         color: _,
    //                         color_view,
    //                         deflection,
    //                         deflection_view,
    //                         color_change,
    //                         color_change_view,
    //                         color_uncertainty,
    //                         color_uncertainty_view,
    //                         covariances,
    //                         covariances_view
    //                     } => Slot::Rgb {
    //                         color: window.target(),
    //                         color_view,
    //                         deflection,
    //                         deflection_view,
    //                         color_change,
    //                         color_change_view,
    //                         color_uncertainty,
    //                         color_uncertainty_view,
    //                         covariances,
    //                         covariances_view
    //                     },
    //                     _ => Slot::Empty,
    //                 },
    //             );
    //         // Negociate and swap.
    //         let new_last_slot = self.nodes.borrow_mut().last_mut().unwrap().negociate_slots(window, suggested_slot, self.resolution, None);
    //         self.last_slot.replace(Some(new_last_slot));
    //     }else{
    //         self.negociate_slots(window);
    //     }
    // }

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
        self.last_slot.replace(Some(slot_b));
    }

    pub fn inspect(&self, inspector: &mut dyn Inspector) {
        // Propagate to nodes.
        for node in self.nodes.borrow_mut().iter_mut() {
            node.inspect(inspector);
        }
    }

    pub fn input(&self, vis_param: &VisualizationParameters) {
        let mut perspective = self.perspective.borrow().clone();

        // Propagate to nodes.
        for node in self.nodes.borrow_mut().iter_mut().rev() {
            perspective = node.input(&perspective, vis_param);
        }
    }

    pub fn render(&self, surface: &Surface, encoder: &mut CommandEncoder, screen: &RenderTexture) {
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

    pub fn post_render(&self, surface: &Surface) {
        for node in self.nodes.borrow_mut().iter_mut() {
            node.post_render(surface);
        }
    }
}
