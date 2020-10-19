//!
//! This module contains several [Nodes](Node) that can be chained to form a [Flow].
//!
#[macro_use]
mod macros;
mod cataract;
mod display;
mod lens;
mod passthrough;
mod retina;
mod rgb_buffer;
mod yuv_buffer;

pub use self::cataract::*;
pub use self::display::*;
pub use self::lens::*;
use self::macros::*;
pub use self::passthrough::*;
pub use self::retina::*;
pub use self::rgb_buffer::*;
pub use self::yuv_buffer::*;

use gfx;
use gfx::traits::FactoryExt;
use gfx::Factory;
use gfx_device_gl;
use gfx_device_gl::Resources;

use super::*;

/// An executable function that implements an aspect of the simulation.
pub trait Node {
    /// Initializes this node.
    fn new(window: &Window) -> Self
    where
        Self: Sized;

    /// Replaces the render target (output) and source texture (input).
    fn update_io(
        &mut self,
        window: &Window,
        source: (Option<NodeSource>, Option<NodeTarget>),
        target_candidate: (Option<NodeSource>, Option<NodeTarget>),
    ) -> (Option<NodeSource>, Option<NodeTarget>);

    /// Set new parameters for this effect
    #[allow(unused_variables)]
    fn update_values(&mut self, window: &Window, values: &ValueMap) {}

    /// Handle input.
    #[allow(unused_variables)]
    fn input(&mut self, head: &Head, gaze: &Gaze) -> Gaze {
        gaze.clone()
    }

    /// Render the node.
    fn render(&mut self, window: &Window);
}
