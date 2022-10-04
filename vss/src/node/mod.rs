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
mod rgb_buffer;
mod slot;
mod test_node;
//TODO-WGPU mod yuv_buffer;
//TODO-WGPU mod vr_compositor;
//TODO-WGPU mod stereo_desktop;
//TODO-WGPU mod variance;
//TODO-WGPU mod peacock;

use wgpu::CommandEncoder;
use wgpu::TextureView;

//TODO-WGPU pub use self::cataract::*;
//TODO-WGPU pub use self::display::*;
//TODO-WGPU pub use self::lens::*;
//TODO-WGPU pub use self::passthrough::*;
//TODO-WGPU pub use self::retina::*;
pub use self::rgb_buffer::*;
pub use self::slot::*;
pub use self::test_node::*;
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
    // fn negociate_slots(&mut self, window: &Window, slots: NodeSlots) -> NodeSlots;

    // fn negociate_slots_wk(&mut self, window: &Window, slots: NodeSlots, _well_known: &WellKnownSlots) -> NodeSlots{
    //     self.negociate_slots(window, slots)
    // }

    /// Set new parameters for this effect
    #[allow(unused_variables)]
    fn update_values(&mut self, window: &Window, values: &ValueMap) {}

    /// Handle input.
    #[allow(unused_variables)]
    fn input(&mut self, perspective: &EyePerspective, vis_param: &VisualizationParameters) -> EyePerspective {
        perspective.clone()
    }

    /// Render the node.
    fn render(&mut self, window: &Window, encoder: &mut CommandEncoder, screen: &RenderTexture);
}

pub fn create_render_pass<'pass>(encoder: &'pass mut CommandEncoder, target: &'pass RenderTexture) -> wgpu::RenderPass<'pass>{
    encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("node_render_pass"),
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view: &target.view,
            resolve_target: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Clear(wgpu::Color {
                    r: 0.5,
                    g: 0.5,
                    b: 0.5,
                    a: 1.0,
                }),
                store: true,
            },
        })],
        depth_stencil_attachment: None,
    })
}