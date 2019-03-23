mod image_device;
mod remote_device;
mod video_device;
mod window_device;

pub use self::image_device::*;
pub use self::remote_device::*;
pub use self::video_device::*;
pub use self::window_device::*;

use std::cell::RefCell;

use gfx;
use gfx_device_gl;

use crate::*;

/// A factory to create pipeline objects from.
pub type DeviceFactory = gfx_device_gl::Factory;

/// An encoder to manipulate the command queue.
pub type DeviceEncoder = gfx::Encoder<gfx_device_gl::Resources, gfx_device_gl::CommandBuffer>;

/// Enum to hold texture-representations for shaders.
#[derive(Clone)]
pub enum DeviceSource {
    Rgb {
        rgba8: gfx::handle::ShaderResourceView<gfx_device_gl::Resources, [f32; 4]>,
    },
    RgbDepth {
        rgba8: gfx::handle::ShaderResourceView<gfx_device_gl::Resources, [f32; 4]>,
        d: gfx::handle::ShaderResourceView<gfx_device_gl::Resources, f32>,
    },
    Yuv {
        y: gfx::handle::ShaderResourceView<gfx_device_gl::Resources, f32>,
        u: gfx::handle::ShaderResourceView<gfx_device_gl::Resources, f32>,
        v: gfx::handle::ShaderResourceView<gfx_device_gl::Resources, f32>,
    },
}

/// Represents properties of eye-tracking data.
#[derive(Debug, Clone)]
pub struct DeviceGaze {
    pub x: f32,
    pub y: f32,
}

pub type DeviceTarget = gfx::handle::RenderTargetView<gfx_device_gl::Resources, ColorFormat>;

/// An input-output simulation device (image, video, camera, ...).
pub trait Device {
    fn factory(&self) -> &RefCell<DeviceFactory>;

    fn encoder(&self) -> &RefCell<DeviceEncoder>;

    fn gaze(&self) -> DeviceGaze;

    fn source(&self) -> &RefCell<DeviceSource>;

    fn target(&self) -> &RefCell<DeviceTarget>;

    fn begin_frame(&self);

    fn end_frame(&self, done: &mut bool);
}
