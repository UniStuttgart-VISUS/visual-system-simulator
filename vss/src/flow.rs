use crate::*;
use std::cell::RefCell;
use cgmath::Matrix4;
use cgmath::Vector3;
use gfx::format::Rgba32F;


/// Represents properties of eye-tracking data.
#[derive(Debug, Clone)]
pub struct Gaze {
    pub x: f32,//remove
    pub y: f32,//remove
    pub direction: Vector3<f32>,
}

/// Represents properties of eye-tracking data.
pub struct Head {
    pub yaw: f32,//remove
    pub pitch: f32,//remove
    pub position: Vector3<f32>,
    pub view: Matrix4<f32>,
    pub proj: Matrix4<f32>,
}

/// A flow encapsulates simulation nodes, i.e., all simulation and rendering.
pub struct Flow {
    nodes: RefCell<Vec<Box<dyn Node>>>,
    last_slot: RefCell<Option<NodeSlots>>,
    pub last_head: RefCell<Head>,
    pub last_gaze: RefCell<Gaze>,
}

impl Flow {
    pub fn new() -> Self {
        Flow {
            nodes: RefCell::new(Vec::new()),
            last_slot: RefCell::new(None),
            last_head: RefCell::new(Head {
                yaw: 0.0,
                pitch: 0.0,
                position: Vector3::new(0.0, 0.0, 0.0),
                view: Matrix4::from_scale(1.0),
                proj: Matrix4::from_scale(1.0),
            }),
            last_gaze: RefCell::new(Gaze {
                x: 0.5,
                y: 0.5,
                direction: Vector3::new(0.0, 0.0, 0.0),
            }),
        }
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
    
    pub fn update_last_slot(&self, window: &Window) {
        if self.last_slot.borrow().is_some() {
            let output = self.last_slot.borrow_mut().as_mut().unwrap().take_output();
            let suggested_slot = 
                NodeSlots::new_io(
                    window,
                    self.last_slot.borrow_mut().as_mut().unwrap().take_input(),
                    match output {
                        Slot::Rgb{
                            color: _,
                            color_view,
                            deflection,
                            deflection_view,
                            color_change,
                            color_change_view,
                            color_uncertainty,
                            color_uncertainty_view
                        } => Slot::Rgb {
                            color: window.target(),
                            color_view,
                            deflection,
                            deflection_view,
                            color_change, 
                            color_change_view, 
                            color_uncertainty, 
                            color_uncertainty_view
                        },
                        _ => Slot::Empty,
                    },
                );
            // Negociate and swap.
            let new_last_slot = self.nodes.borrow_mut().last_mut().unwrap().negociate_slots(window, suggested_slot);
            self.last_slot.replace(Some(new_last_slot));
        }else{
            self.negociate_slots(window);
        }
    }

    pub fn negociate_slots(&self, window: &Window) {
        let mut slot_a = NodeSlots::new(window);
        let mut slot_b = NodeSlots::new(window);
        let nodes_len = self.nodes.borrow().len();
        for (idx, node) in self.nodes.borrow_mut().iter_mut().enumerate() {

            let suggested_slot = if idx + 1 == nodes_len {
                // Suggest window as final output.
                let mut factory = window.factory().borrow_mut();

                let (width, height, ..) = window.target().get_dimensions();

                let (deflection, deflection_view) = create_texture_render_target::<Rgba32F>(
                    &mut factory,
                    width as u32,
                    height as u32,
                );
                let (color_change, color_change_view) = create_texture_render_target::<Rgba32F>(
                    &mut factory,
                    width as u32,
                    height as u32,
                );
                let (color_uncertainty, color_uncertainty_view) = create_texture_render_target::<Rgba32F>(
                    &mut factory,
                    width as u32,
                    height as u32,
                );

                drop(factory);

                let output_slot = Slot::Rgb {
                    color: window.target(),
                    color_view: None,
                    deflection,
                    deflection_view,
                    color_change, 
                    color_change_view, 
                    color_uncertainty, 
                    color_uncertainty_view
                };

                NodeSlots::new_io(
                    window,
                    slot_b.take_output(),
                    output_slot
                )
            } else {
                // Suggest reusing output of the pre-predecessor.
                NodeSlots::new_io(window, slot_b.take_output(), slot_a.take_output())
            };
            // Negociate and swap.
            slot_a = node.negociate_slots(window, suggested_slot);
            std::mem::swap(&mut slot_a, &mut slot_b);
        }
        self.last_slot.replace(Some(slot_b));
    }

    pub fn update_values(&self, window: &Window, values: &ValueMap) {
        // Propagate to nodes.
        for node in self.nodes.borrow_mut().iter_mut() {
            node.update_values(window, &values);
        }
    }

    pub fn input(&self, vis_param: &VisualizationParameters) {
        let mut gaze = self.last_gaze.borrow().clone();
        // Propagate to nodes.
        for node in self.nodes.borrow_mut().iter_mut().rev() {
            gaze = node.input(&self.last_head.borrow(), &gaze, vis_param);
        }
        self.last_gaze.replace(gaze);
    }

    pub fn render(&self, window: &Window) {
        // Render all nodes.
        for node in self.nodes.borrow_mut().iter_mut() {
            node.render(window);
        }
    }
}
