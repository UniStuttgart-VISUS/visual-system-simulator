//!
//! This module contains several [Nodes](Node) that can be chained to form a [Flow].
//!
mod cataract;
mod display;
mod eye_control;
mod gui_overlay;
mod lens;
mod passthrough;
mod peacock;
mod retina;
mod rgb_buffer;
mod slot;
mod variance;
mod vis_overlay;
mod yuv_buffer;

use wgpu::util::DeviceExt;
use wgpu::BindGroupLayout;
use wgpu::ColorTargetState;
use wgpu::CommandEncoder;
use wgpu::DepthStencilState;
use wgpu::RenderPipeline;
use wgpu::ShaderModule;

use cgmath::Matrix4;

pub use self::cataract::*;
pub use self::display::*;
pub use self::eye_control::*;
pub use self::gui_overlay::*;
pub use self::lens::*;
pub use self::passthrough::*;
pub use self::peacock::*;
pub use self::retina::*;
pub use self::rgb_buffer::*;
pub use self::slot::*;
pub use self::variance::*;
pub use self::vis_overlay::*;
pub use self::yuv_buffer::*;

use super::*;

/// An executable function that implements an aspect of the simulation.
pub trait Node {
    // Returns the node name.
    fn name(&self) -> &'static str;

    /// Negociates input and output for this node (source texture and render target),
    /// possibly re-using suggested `slots` (for efficiency).
    fn negociate_slots(
        &mut self,
        surface: &Surface,
        slots: NodeSlots,
        original_image: &mut Option<Texture>,
    ) -> NodeSlots;

    /// Set new parameters for this effect
    #[allow(unused_variables)]
    fn inspect(&mut self, inspector: &dyn Inspector) {}

    /// Handle input.
    #[allow(unused_variables)]
    fn input(&mut self, eye: &EyeInput, mouse: &MouseInput) -> EyeInput {
        eye.clone()
    }

    /// Issue render commands for the node.
    fn render(
        &mut self,
        surface: &Surface,
        encoder: &mut CommandEncoder,
        screen: Option<&RenderTexture>,
    );

    /// Invoked after all rendering commands have completed. (TODO: rename to on_frame_complete)
    #[allow(unused_variables)]
    fn post_render(&mut self, surface: &Surface) {}

    fn as_ui_mut(&mut self) -> Option<&'_ mut GuiOverlay> {
        None
    }
}

impl Node for Box<dyn Node> {
    fn name(&self) -> &'static str {
        self.as_ref().name()
    }

    fn negociate_slots(
        &mut self,
        surface: &Surface,
        slots: NodeSlots,
        original_image: &mut Option<Texture>,
    ) -> NodeSlots {
        self.as_mut()
            .negociate_slots(surface, slots, original_image)
    }

    fn inspect(&mut self, inspector: &dyn Inspector) {
        self.as_mut().inspect(inspector);
    }

    fn input(&mut self, eye: &EyeInput, mouse: &MouseInput) -> EyeInput {
        self.as_mut().input(eye, mouse)
    }
    fn render(
        &mut self,
        surface: &Surface,
        encoder: &mut CommandEncoder,
        screen: Option<&RenderTexture>,
    ) {
        self.as_mut().render(surface, encoder, screen);
    }

    /// Invoked after all rendering commands have completed. (TODO: rename to on_frame_complete)
    #[allow(unused_variables)]
    fn post_render(&mut self, surface: &Surface) {
        self.as_mut().post_render(surface);
    }

    fn as_ui_mut(&mut self) -> Option<&'_ mut GuiOverlay> {
        self.as_mut().as_ui_mut()
    }
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
        blended_color_state(COLOR_FORMAT),
        simple_color_state(HIGHP_FORMAT),
        simple_color_state(HIGHP_FORMAT),
        simple_color_state(HIGHP_FORMAT),
        simple_color_state(HIGHP_FORMAT),
    ]
}

pub fn simple_depth_state(format: wgpu::TextureFormat) -> Option<DepthStencilState> {
    Some(DepthStencilState {
        format,
        depth_write_enabled: true,
        depth_compare: wgpu::CompareFunction::Less,
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
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label,
        bind_group_layouts,
        push_constant_ranges: &[],
    });

    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label,
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
            module: modules[0],
            entry_point: entry_points[0],
            buffers: &[],
        },
        fragment: Some(wgpu::FragmentState {
            module: modules[1],
            entry_point: entry_points[1],
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
        // If the pipeline will be used with a multiview render pass, this
        // indicates how many array layers the attachments will have.
        multiview: None,
    })
}
