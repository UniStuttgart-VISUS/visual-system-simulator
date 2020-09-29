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
    fn input(&mut self, gaze: &DeviceGaze) -> DeviceGaze {
        gaze.clone()
    }

    /// Render the node.
    fn render(&mut self, window: &Window);
}

#[macro_export]
macro_rules! unimplemented_node {
    ($name:ident) => {
        use $crate::*;

        pub struct $name;

        impl Node for $name {
            fn new(_window: &Window) -> Self {
                unimplemented!();
            }

            fn update_io(
                &mut self,
                _window: &Window,
                _source: (Option<DeviceSource>, Option<DeviceTarget>),
                _target_candidate: (Option<DeviceSource>, Option<DeviceTarget>),
            ) -> (Option<DeviceSource>, Option<DeviceTarget>) {
                unimplemented!();
            }

            fn update_values(&mut self, _window: &Window, _values: &ValueMap) {
                unimplemented!();
            }

            fn input(&mut self, gaze: &DeviceGaze) -> DeviceGaze {
                unimplemented!();
            }

            fn render(&mut self, _window: &Window) {
                unimplemented!();
            }
        }
    };
}
