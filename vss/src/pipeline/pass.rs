use std::collections::HashMap;

use gfx;
use gfx_device_gl;
use gfx_device_gl::CommandBuffer;
use gfx_device_gl::Resources;

use crate::devices::DeviceGaze;
use crate::devices::DeviceSource;
use crate::devices::DeviceTarget;

#[derive(Debug)]
pub enum Value {
    Bool(bool),
    Number(f64),
    Image(String),
}

impl Value {
    pub fn as_bool(&self) -> Option<bool> {
        match *self {
            Value::Bool(b) => Some(b),
            _ => None,
        }
    }

    pub fn as_f64(&self) -> Option<f64> {
        match *self {
            Value::Number(f) => Some(f),
            _ => None,
        }
    }

    pub fn as_img(&self) -> Option<&str> {
        match *self {
            Value::Image(ref s) => Some(s),
            _ => None,
        }
    }
}

pub type ValueMap = HashMap<String, Value>;

/// An executable function that implements an aspect of the simulation pipeline.
///
/// Initialize with `build(...)`, then apply with `render(...)`.
///
/// The texture this pass is applied to and where the output will be written
/// is determined by the RenderContext passed to `build(...)`.
pub trait Pass {
    /// Initialize this pass.
    fn build(&mut self, factory: &mut gfx_device_gl::Factory, vertex_data: Option<[f32; 48]>);

    /// Replaces the output (render target) and input (source texture).
    fn update_io(
        &mut self,
        target: &DeviceTarget,
        target_size: (u32, u32),
        source: &DeviceSource,
        source_sampler: &gfx::handle::Sampler<Resources>,
        source_size: (u32, u32),
    );

    /// Set new parameters for this effect
    fn update_params(&mut self, factory: &mut gfx_device_gl::Factory, values: &ValueMap);

    /// Apply the pass.
    fn render(&mut self, encoder: &mut gfx::Encoder<Resources, CommandBuffer>, gaze: &DeviceGaze);
}
