//!
//! This module contains several [Nodes](Node) that can be chained to form a [Flow].
//!
#[macro_use]
//TODO-WGPU mod macros;
//TODO-WGPU mod cataract;
//TODO-WGPU mod display;
//TODO-WGPU mod lens;
//TODO-WGPU mod passthrough;
//TODO-WGPU mod retina;
//TODO-WGPU mod rgb_buffer;
//TODO-WGPU mod slot;
//TODO-WGPU mod yuv_buffer;
//TODO-WGPU mod vr_compositor;
//TODO-WGPU mod stereo_desktop;
//TODO-WGPU mod variance;
//TODO-WGPU mod peacock;

//TODO-WGPU pub use self::cataract::*;
//TODO-WGPU pub use self::display::*;
//TODO-WGPU pub use self::lens::*;
//TODO-WGPU pub use self::passthrough::*;
//TODO-WGPU pub use self::retina::*;
//TODO-WGPU pub use self::rgb_buffer::*;
//TODO-WGPU pub use self::slot::*;
//TODO-WGPU pub use self::yuv_buffer::*;
//TODO-WGPU pub use self::vr_compositor::*;
//TODO-WGPU pub use self::stereo_desktop::*;
//TODO-WGPU pub use self::variance::*;
//TODO-WGPU pub use self::peacock::*;

//TODO-WGPU use self::macros::*;

//TODO-WGPU use gfx;
//TODO-WGPU use gfx::traits::FactoryExt;
//TODO-WGPU use gfx::Factory;
//TODO-WGPU use gfx_device_gl;
//TODO-WGPU use gfx_device_gl::Resources;

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
