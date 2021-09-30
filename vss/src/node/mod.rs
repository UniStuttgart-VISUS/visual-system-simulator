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
mod slot;
mod yuv_buffer;
mod vr_compositor;
mod stereo_desktop;

pub use self::cataract::*;
pub use self::display::*;
pub use self::lens::*;
pub use self::passthrough::*;
pub use self::retina::*;
pub use self::rgb_buffer::*;
pub use self::slot::*;
pub use self::yuv_buffer::*;
pub use self::vr_compositor::*;
pub use self::stereo_desktop::*;

use self::macros::*;

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

    /// Negociates input and output for this node (source texture and render target),
    /// possibly re-using suggested `slots` (for efficiency).
    fn negociate_slots(&mut self, window: &Window, slots: NodeSlots) -> NodeSlots;

    fn negociate_slots_wk(&mut self, window: &Window, slots: NodeSlots, _well_known: &WellKnownSlots) -> NodeSlots{
        self.negociate_slots(window, slots)
    }

    /// Set new parameters for this effect
    #[allow(unused_variables)]
    fn update_values(&mut self, window: &Window, values: &ValueMap) {}

    /// Handle input.
    #[allow(unused_variables)]
    fn input(&mut self, perspective: &EyePerspective, vis_param: &VisualizationParameters) -> EyePerspective {
        perspective.clone()
    }

    /// Render the node.
    fn render(&mut self, window: &Window);
}
