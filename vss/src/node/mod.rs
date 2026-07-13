//!
//! This module contains several [Nodes](Node) that can be chained to form a [Flow].
//!
pub mod cataract;
mod display;
pub mod eye_control;
pub mod lens;
pub mod peacock;
pub mod retina;
mod rgb_buffer;
mod slot;
pub mod variance;
pub mod vis_overlay;
mod yuv_buffer;

use wgpu::util::DeviceExt;
use wgpu::BindGroupLayout;
use wgpu::ColorTargetState;
use wgpu::CommandEncoder;
use wgpu::DepthStencilState;
use wgpu::PipelineCompilationOptions;
use wgpu::RenderPipeline;
use wgpu::ShaderModule;

use cgmath::Matrix4;

pub use self::cataract::{Cataract, CataractConfig};
pub use self::display::*;
pub use self::eye_control::{EyeControl, EyeControlConfig};
pub use self::lens::{Lens, LensConfig};
pub use self::peacock::{PeacockCB, PeacockConfig};
pub use self::retina::{Retina, RetinaConfig};
pub use self::rgb_buffer::*;
pub use self::slot::*;
pub use self::variance::{VarianceConfig, VarianceMeasure};
pub use self::vis_overlay::{VisOverlay, VisOverlayConfig};
pub use self::yuv_buffer::*;

use super::*;
use std::any::Any;

/// Converts a struct to `&[u8]`.
///
/// # Safety
/// Padding bytes in the type may be uninitialized. The caller must ensure that
/// reading every byte of the value is valid, for example by using `#[repr(packed)]`.
unsafe fn any_as_u8_slice<T: Sized>(value: &T) -> &[u8] {
    std::slice::from_raw_parts((value as *const T).cast::<u8>(), std::mem::size_of::<T>())
}

bitflags::bitflags! {
    #[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
    pub struct NodeChanges: u8 {
        const OUTPUT = 1 << 0;
        const SLOTS = 1 << 1;
    }
}

impl NodeChanges {
    pub fn from_output_slots(output: bool, slots: bool) -> Self {
        let mut result = Self::empty();
        result.set(Self::OUTPUT, output);
        result.set(Self::SLOTS, slots);
        result.normalized()
    }

    pub fn normalized(mut self) -> Self {
        if self.contains(Self::SLOTS) {
            self.insert(Self::OUTPUT);
        }
        self
    }
}

/// An executable function that implements an aspect of the simulation.
pub trait Node: Any {
    // Returns the node name.
    fn name(&self) -> &'static str;

    /// Negociates input and output for this node (source texture and render target),
    /// possibly re-using suggested `slots` (for efficiency).
    fn negociate_slots(
        &mut self,
        context: &RenderContext,
        slots: NodeSlots,
        original_image: &mut Option<Texture>,
    ) -> NodeSlots;

    fn configure(&mut self) -> NodeChanges {
        NodeChanges::empty()
    }

    /// Handle input.
    #[allow(unused_variables)]
    fn input(&mut self, eye: &EyeInput, mouse: &MouseInput) -> (EyeInput, NodeChanges) {
        let _ = mouse;
        (eye.clone(), NodeChanges::empty())
    }

    /// Issue render commands for the node.
    fn render(
        &mut self,
        context: &RenderContext,
        encoder: &mut CommandEncoder,
        screen: Option<&RenderTexture>,
    );

    /// Invoked after all rendering commands have completed. (TODO: rename to on_frame_complete)
    #[allow(unused_variables)]
    fn post_render(&mut self, context: &RenderContext) {}
}

pub struct ShaderUniforms<T> {
    pub data: T,
    buffer: wgpu::Buffer,
    pub bind_group_layout: wgpu::BindGroupLayout,
    pub bind_group: wgpu::BindGroup,
}

impl<T> ShaderUniforms<T> {
    pub fn new(device: &wgpu::Device, data: T) -> Self {
        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("uniforms_buffer"),
            contents: unsafe { any_as_u8_slice(&data) },
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
            label: Some("uniforms_bind_group_layout"),
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: buffer.as_entire_binding(),
            }],
            label: Some("uniforms_bind_group"),
        });

        ShaderUniforms {
            data,
            buffer,
            bind_group_layout,
            bind_group,
        }
    }

    pub fn upload(&self, queue: &wgpu::Queue) {
        queue.write_buffer(&self.buffer, 0, unsafe { any_as_u8_slice(&self.data) });
    }
}

pub fn simple_color_state(format: wgpu::TextureFormat) -> Option<ColorTargetState> {
    Some(ColorTargetState {
        format,
        blend: None,
        write_mask: wgpu::ColorWrites::ALL,
    })
}

pub fn blended_color_state(format: wgpu::TextureFormat) -> Option<ColorTargetState> {
    Some(ColorTargetState {
        format,
        blend: Some(wgpu::BlendState {
            color: wgpu::BlendComponent::REPLACE,
            alpha: wgpu::BlendComponent::REPLACE,
        }),
        write_mask: wgpu::ColorWrites::ALL,
    })
}

pub fn all_color_states() -> [Option<ColorTargetState>; 5] {
    [
        simple_color_state(COLOR_FORMAT),
        simple_color_state(METRICS_FORMAT),
        simple_color_state(METRICS_FORMAT),
        simple_color_state(METRICS_FORMAT),
        simple_color_state(METRICS_FORMAT),
    ]
}

pub fn single_color_state(format: wgpu::TextureFormat) -> [Option<ColorTargetState>; 1] {
    [simple_color_state(format)]
}

pub fn metrics_ab_color_states() -> [Option<ColorTargetState>; 2] {
    [
        simple_color_state(METRICS_FORMAT),
        simple_color_state(METRICS_FORMAT),
    ]
}

pub fn metrics_cd_color_states() -> [Option<ColorTargetState>; 2] {
    [
        simple_color_state(METRICS_FORMAT),
        simple_color_state(METRICS_FORMAT),
    ]
}

pub fn simple_depth_state(format: wgpu::TextureFormat) -> Option<DepthStencilState> {
    Some(DepthStencilState {
        format,
        depth_write_enabled: Some(true),
        depth_compare: Some(wgpu::CompareFunction::Less),
        stencil: wgpu::StencilState::default(),
        bias: wgpu::DepthBiasState::default(),
    })
}

pub fn create_render_pipeline(
    device: &wgpu::Device,
    modules: &[&ShaderModule; 2],
    entry_points: &[&str; 2],
    bind_group_layouts: &[&BindGroupLayout],
    color_targets: &[Option<ColorTargetState>],
    depth_tagret: Option<DepthStencilState>,
    label: Option<&str>,
) -> RenderPipeline {
    let bind_group_layouts: Vec<Option<&BindGroupLayout>> =
        bind_group_layouts.iter().copied().map(Some).collect();
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label,
        bind_group_layouts: &bind_group_layouts,
        immediate_size: 0,
    });

    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label,
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
            module: modules[0],
            entry_point: Some(entry_points[0]),
            compilation_options: PipelineCompilationOptions::default(),
            buffers: &[],
        },
        fragment: Some(wgpu::FragmentState {
            module: modules[1],
            entry_point: Some(entry_points[1]),
            compilation_options: PipelineCompilationOptions::default(),
            targets: color_targets,
        }),
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            strip_index_format: None,
            front_face: wgpu::FrontFace::Ccw,
            cull_mode: Some(wgpu::Face::Back),
            // Setting this to anything other than Fill requires Features::POLYGON_MODE_LINE
            // or Features::POLYGON_MODE_POINT
            polygon_mode: wgpu::PolygonMode::Fill,
            // Requires Features::DEPTH_CLIP_CONTROL
            unclipped_depth: false,
            // Requires Features::CONSERVATIVE_RASTERIZATION
            conservative: false,
        },
        depth_stencil: depth_tagret,
        multisample: wgpu::MultisampleState {
            count: 1,
            mask: !0,
            alpha_to_coverage_enabled: false,
        },
        multiview_mask: None,
        cache: None,
    })
}
