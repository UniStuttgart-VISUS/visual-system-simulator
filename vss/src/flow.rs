use crate::*;
use std::cell::RefCell;

pub type ColorFormat = (gfx::format::R8_G8_B8_A8, gfx::format::Unorm);
pub type DepthFormat = (gfx::format::R32, gfx::format::Float);

/// Represents texture views for shaders.
#[derive(Clone, Debug)]
pub enum NodeSource {
    Rgb {
        width: u32,
        height: u32,
        rgba8: gfx::handle::ShaderResourceView<gfx_device_gl::Resources, [f32; 4]>,
    },
    RgbDepth {
        width: u32,
        height: u32,
        rgba8: gfx::handle::ShaderResourceView<gfx_device_gl::Resources, [f32; 4]>,
        d: gfx::handle::ShaderResourceView<gfx_device_gl::Resources, f32>,
    },
}

pub type NodeTarget = gfx::handle::RenderTargetView<gfx_device_gl::Resources, ColorFormat>;

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

    pub fn update_io(&self, window: &Window) {
        let mut last_targets: [(Option<NodeSource>, Option<NodeTarget>); 2] =
            [(None, None), (None, None)];
        let nodes_len = self.nodes.borrow().len();
        for (idx, node) in self.nodes.borrow_mut().iter_mut().enumerate() {
            // Determine source.
            let source = last_targets[0].clone();
            let target_candidate = if idx + 1 == nodes_len {
                // Suggest window as final target.
                (None, Some(window.target().clone()))
            } else {
                if let (Some(source), Some(target)) = &last_targets[1] {
                    // Suggest reusing the pre-predecessor's target.
                    (Some(source.clone()), Some(target.clone()))
                } else if let Some(source) = &source.0 {
                    // Guess target, based on source.
                    let (width, height) = match *source {
                        NodeSource::Rgb { width, height, .. } => (width, height),
                        NodeSource::RgbDepth { width, height, .. } => (width, height),
                    };
                    let (target_view, target) = create_texture_render_target::<ColorFormat>(
                        &mut window.factory().borrow_mut(),
                        width,
                        height,
                    );
                    (
                        Some(NodeSource::Rgb {
                            width,
                            height,
                            rgba8: target_view,
                        }),
                        Some(target),
                    )
                } else {
                    // No suggestion (can cause update-aborting errors).
                    (None, None)
                }
            };
            // Chain targets and update.
            last_targets.swap(1, 0);
            last_targets[0] = node.update_io(window, source, target_candidate);
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
