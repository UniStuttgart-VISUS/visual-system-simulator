mod node;
mod texture;
#[macro_use]
mod utils;
mod value;

pub use self::node::*;
pub use self::texture::*;
pub use self::utils::*;
pub use self::value::*;

use gfx::traits::FactoryExt;
use gfx::Factory;
use std::cell::RefCell;

use crate::window::*;

/// A factory to create pipeline objects from.
pub type DeviceFactory = gfx_device_gl::Factory;

/// An encoder to manipulate the command queue.
pub type DeviceEncoder = gfx::Encoder<gfx_device_gl::Resources, gfx_device_gl::CommandBuffer>;

/// Represents properties of eye-tracking data.
#[derive(Debug, Clone)]
pub struct DeviceGaze {
    pub x: f32,
    pub y: f32,
}

/// Enum to hold texture-representations for shaders.
#[derive(Clone, Debug)]
pub enum DeviceSource {
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
    Yuv {
        width: u32,
        height: u32,
        y: gfx::handle::ShaderResourceView<gfx_device_gl::Resources, f32>,
        u: gfx::handle::ShaderResourceView<gfx_device_gl::Resources, f32>,
        v: gfx::handle::ShaderResourceView<gfx_device_gl::Resources, f32>,
    },
}

pub type RgbSurfaceFormat = gfx::format::R8_G8_B8_A8; //TODO: remove
pub type YuvSurfaceFormat = gfx::format::R8; //TODO: remove
pub type ColorFormat = (RgbSurfaceFormat, gfx::format::Unorm);

pub type DeviceTarget = gfx::handle::RenderTargetView<gfx_device_gl::Resources, ColorFormat>;

// A buffer representing color information.
pub struct RGBBuffer {
    pub pixels_rgb: Box<[u8]>,
    pub width: usize,
    pub height: usize,
}

/// The pipeline encapsulates the simulation and rendering system, i.e., all rendering nodes.
pub struct Pipeline {
    nodes: RefCell<Vec<Box<dyn Node>>>,
}

impl Pipeline {
    pub fn new() -> Self {
        Pipeline {
            nodes: RefCell::new(Vec::new()),
        }
    }

    pub fn add_node(&mut self, node: Box<dyn Node>) {
        self.nodes.borrow_mut().push(node);
    }

    pub fn replace_at(&mut self, index: usize, node: Box<dyn Node>) {
        self.nodes.borrow_mut()[index] = node;
    }

    pub fn update_io(&self, window: &Window) {
        let mut factory = window.factory().borrow_mut();
        let mut last_targets: [(Option<DeviceSource>, Option<DeviceTarget>); 2] =
            [(None, None), (None, None)];
        let nodes_len = self.nodes.borrow().len();
        for (idx, node) in self.nodes.borrow_mut().iter_mut().enumerate() {
            // Determine source.
            let source = last_targets[0].0.clone();
            let target_candidate = if idx + 1 == nodes_len {
                // Suggest window as final target.
                (None, Some(window.target().clone()))
            } else {
                if let (Some(source), Some(target)) = &last_targets[1] {
                    // Suggest reusing the pre-predecessor's target.
                    (Some(source.clone()), Some(target.clone()))
                } else if let Some(source) = &source {
                    // Guess target, based on source.
                    let (width, height) = match *source {
                        DeviceSource::Rgb { width, height, .. } => (width, height),
                        DeviceSource::RgbDepth { width, height, .. } => (width, height),
                        DeviceSource::Yuv { width, height, .. } => (width, height),
                    };

                    let (_, source, target) = factory
                        .create_render_target(width as u16, height as u16)
                        .unwrap();
                    (
                        Some(DeviceSource::Rgb {
                            width: width as u32,
                            height: height as u32,
                            rgba8: source,
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
            last_targets[0] = node.update_io(&mut factory, source, target_candidate);
        }
    }

    pub fn update_values(&self, window: &Window, values: &ValueMap) {
        let mut factory = window.factory().borrow_mut();

        // Propagate to nodes.
        for node in self.nodes.borrow_mut().iter_mut() {
            node.update_values(&mut factory, &values);
        }
    }

    pub fn input(&self, gaze: &DeviceGaze) {
        let mut gaze = gaze.clone();
        // Propagate to nodes.
        for node in self.nodes.borrow_mut().iter_mut().rev() {
            gaze = node.input(&gaze);
        }
    }

    pub fn render(&self, window: &Window) {
        let mut encoder = window.encoder().borrow_mut();

        // Render all nodes.
        for node in self.nodes.borrow_mut().iter_mut() {
            node.render(&mut encoder);
        }
    }
}
