//!
//! This module contains several [nodes](Node) that can be chained to form a [pipeline](Pipeline).
//!

mod cataract;
mod display;
mod download_rgb_buffer;
mod lens;
mod passthrough;
mod retina;
mod upload_rgb_buffer;
mod upload_yuv_buffer;

pub use self::cataract::*;
pub use self::display::*;
pub use self::download_rgb_buffer::*;
pub use self::lens::*;
pub use self::passthrough::*;
pub use self::retina::*;
pub use self::upload_rgb_buffer::*;
pub use self::upload_yuv_buffer::*;

pub use gfx;
pub use gfx_device_gl;
pub use gfx_device_gl::CommandBuffer;
pub use gfx_device_gl::Resources;

use super::*;

/// An executable function that implements an aspect of the simulation pipeline.
///
/// Initialize with `build(...)`, then apply with `render(...)`.
///
/// The texture this pass is applied to and where the output will be written
/// is determined by the RenderContext passed to `build(...)`.
pub trait Node {
    fn new(window: &Window) -> Self
    where
        Self: Sized;

    /// Replaces the render target (output) and source texture (input).
    fn update_io(
        &mut self,
        window: &Window,
        source: (Option<DeviceSource>, Option<DeviceTarget>),
        target_candidate: (Option<DeviceSource>, Option<DeviceTarget>),
    ) -> (Option<DeviceSource>, Option<DeviceTarget>);

    /// Set new parameters for this effect
    #[allow(unused_variables)]
    fn update_values(&mut self, window: &Window, values: &ValueMap) {}

    /// Handle input.
    fn input(&mut self, _head: &Head, gaze: &DeviceGaze) -> DeviceGaze {
        gaze.clone()
    }

    /// Render the node.
    fn render(&mut self, window: &Window);
}
