use crate::*;
use std::cell::RefCell;

/// Represents properties of eye-tracking data.
#[derive(Debug, Clone)]
pub struct Gaze {
    pub x: f32,
    pub y: f32,
}

/// Represents properties of eye-tracking data.
pub struct Head {
    pub yaw: f32,
    pub pitch: f32,
}

/// A flow encapsulates simulation nodes, i.e., all simulation and rendering.
pub struct Flow {
    nodes: RefCell<Vec<Box<dyn Node>>>,
}

impl Flow {
    pub fn new() -> Self {
        Flow {
            nodes: RefCell::new(Vec::new()),
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

    pub fn negociate_slots(&self, window: &Window) {
        let mut slot_a = NodeSlots::new(window);
        let mut slot_b = NodeSlots::new(window);
        let nodes_len = self.nodes.borrow().len();
        for (idx, node) in self.nodes.borrow_mut().iter_mut().enumerate() {
            let suggested_slot = if idx + 1 == nodes_len {
                // Suggest window as final output.
                NodeSlots::new_io(
                    window,
                    slot_b.take_output(),
                    Slot::Rgb {
                        color: window.target(),
                        color_view: None,
                    },
                )
            } else {
                // Suggest reusing output of the pre-predecessor.
                NodeSlots::new_io(window, slot_b.take_output(), slot_a.take_output())
            };
            // Negociate and swap.
            slot_a = node.negociate_slots(window, suggested_slot);
            std::mem::swap(&mut slot_a, &mut slot_b);
        }
    }

    pub fn update_values(&self, window: &Window, values: &ValueMap) {
        // Propagate to nodes.
        for node in self.nodes.borrow_mut().iter_mut() {
            node.update_values(window, &values);
        }
    }

    pub fn input(&self, head: &Head, gaze: &Gaze) {
        let mut gaze = gaze.clone();
        // Propagate to nodes.
        for node in self.nodes.borrow_mut().iter_mut().rev() {
            gaze = node.input(head, &gaze);
        }
    }

    pub fn render(&self, window: &Window) {
        // Render all nodes.
        for node in self.nodes.borrow_mut().iter_mut() {
            node.render(window);
        }
    }
}
