use wgpu::{BindGroup, RenderPassColorAttachment, RenderPassDepthStencilAttachment};

use super::*;

pub static COLOR_FORMAT: wgpu::TextureFormat = if cfg!(target_arch = "wasm32") {
    wgpu::TextureFormat::Rgba8Unorm
} else if cfg!(target_os = "android") {
    wgpu::TextureFormat::Rgba8UnormSrgb
} else {
    wgpu::TextureFormat::Bgra8Unorm
};
pub static HIGHP_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba32Float;
pub static METRICS_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba16Float;
pub static DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;

pub static CLEAR_COLOR: wgpu::Color = wgpu::Color {
    r: 0.5,
    g: 0.5,
    b: 0.5,
    a: 1.0,
};

#[derive(Clone)]
pub struct PackedMetricsTargets {
    pub rt_metrics_a: RenderTexture,
    pub rt_metrics_b: RenderTexture,
    pub rt_metrics_c: RenderTexture,
    pub rt_metrics_d: RenderTexture,
}

impl PackedMetricsTargets {
    pub fn new(device: &wgpu::Device, node_name: &str) -> Self {
        Self {
            rt_metrics_a: RenderTexture::empty_metrics(
                device,
                Some(format!("{}{}", node_name, " rt_metrics_a (placeholder)").as_str()),
            ),
            rt_metrics_b: RenderTexture::empty_metrics(
                device,
                Some(format!("{}{}", node_name, " rt_metrics_b (placeholder)").as_str()),
            ),
            rt_metrics_c: RenderTexture::empty_metrics(
                device,
                Some(format!("{}{}", node_name, " rt_metrics_c (placeholder)").as_str()),
            ),
            rt_metrics_d: RenderTexture::empty_metrics(
                device,
                Some(format!("{}{}", node_name, " rt_metrics_d (placeholder)").as_str()),
            ),
        }
    }

    pub fn ab_attachments(&self) -> [Option<RenderPassColorAttachment<'_>>; 2] {
        [
            self.rt_metrics_a.to_color_attachment(Some(CLEAR_COLOR)),
            self.rt_metrics_b.to_color_attachment(Some(CLEAR_COLOR)),
        ]
    }

    pub fn cd_attachments(&self) -> [Option<RenderPassColorAttachment<'_>>; 2] {
        [
            self.rt_metrics_c.to_color_attachment(Some(CLEAR_COLOR)),
            self.rt_metrics_d.to_color_attachment(Some(CLEAR_COLOR)),
        ]
    }
}

#[derive(Clone)]
pub struct ColorTargets {
    pub rt_color: RenderTexture,
    pub rt_deflection: RenderTexture,
    pub rt_color_change: RenderTexture,
    pub rt_color_uncertainty: RenderTexture,
    pub rt_covariances: RenderTexture,
}

impl ColorTargets {
    pub fn new(device: &wgpu::Device, node_name: &str) -> Self {
        Self {
            rt_color: RenderTexture::empty_color(
                device,
                Some(format!("{}{}", node_name, " rt_color (placeholder)").as_str()),
            ),
            rt_deflection: RenderTexture::empty_metrics(
                device,
                Some(format!("{}{}", node_name, " rt_metrics_a (placeholder)").as_str()),
            ),
            rt_color_change: RenderTexture::empty_metrics(
                device,
                Some(format!("{}{}", node_name, " rt_metrics_b (placeholder)").as_str()),
            ),
            rt_color_uncertainty: RenderTexture::empty_metrics(
                device,
                Some(format!("{}{}", node_name, " rt_metrics_c (placeholder)").as_str()),
            ),
            rt_covariances: RenderTexture::empty_metrics(
                device,
                Some(format!("{}{}", node_name, " rt_metrics_d (placeholder)").as_str()),
            ),
        }
    }

    pub fn color_attachments<'a>(
        &'a self,
        screen: Option<&'a RenderTexture>,
    ) -> [Option<RenderPassColorAttachment<'a>>; 1] {
        [screen
            .unwrap_or(&self.rt_color)
            .to_color_attachment(Some(CLEAR_COLOR))]
    }
}

#[derive(Clone)]
pub struct ColorDepthTargets {
    pub rt_color: RenderTexture,
    pub rt_depth: RenderTexture,
    pub rt_deflection: RenderTexture,
    pub rt_color_change: RenderTexture,
    pub rt_color_uncertainty: RenderTexture,
    pub rt_covariances: RenderTexture,
}

impl ColorDepthTargets {
    pub fn new(device: &wgpu::Device, node_name: &str) -> Self {
        Self {
            rt_color: RenderTexture::empty_color(
                device,
                Some(format!("{}{}", node_name, " rt_color (placeholder)").as_str()),
            ),
            rt_depth: RenderTexture::empty_depth(
                device,
                Some(format!("{}{}", node_name, " rt_depth (placeholder)").as_str()),
            ),
            rt_deflection: RenderTexture::empty_metrics(
                device,
                Some(format!("{}{}", node_name, " rt_metrics_a (placeholder)").as_str()),
            ),
            rt_color_change: RenderTexture::empty_metrics(
                device,
                Some(format!("{}{}", node_name, " rt_metrics_b (placeholder)").as_str()),
            ),
            rt_color_uncertainty: RenderTexture::empty_metrics(
                device,
                Some(format!("{}{}", node_name, " rt_metrics_c (placeholder)").as_str()),
            ),
            rt_covariances: RenderTexture::empty_metrics(
                device,
                Some(format!("{}{}", node_name, " rt_metrics_d (placeholder)").as_str()),
            ),
        }
    }

    pub fn color_attachments<'a>(
        &'a self,
        screen: Option<&'a RenderTexture>,
    ) -> [Option<RenderPassColorAttachment<'a>>; 1] {
        [screen
            .unwrap_or(&self.rt_color)
            .to_color_attachment(Some(CLEAR_COLOR))]
    }

    pub fn depth_attachment(&self) -> Option<RenderPassDepthStencilAttachment<'_>> {
        self.rt_depth.to_depth_attachment(Some(1.0))
    }
}

#[derive(Clone)]
pub struct ColorMetricsTargets {
    pub rt_color: RenderTexture,
    pub metrics: PackedMetricsTargets,
}

impl ColorMetricsTargets {
    pub fn new(device: &wgpu::Device, node_name: &str) -> Self {
        Self {
            rt_color: RenderTexture::empty_color(
                device,
                Some(format!("{}{}", node_name, " rt_color (placeholder)").as_str()),
            ),
            metrics: PackedMetricsTargets::new(device, node_name),
        }
    }

    pub fn color_attachments<'a>(
        &'a self,
        screen: Option<&'a RenderTexture>,
    ) -> [Option<RenderPassColorAttachment<'a>>; 1] {
        [screen
            .unwrap_or(&self.rt_color)
            .to_color_attachment(Some(CLEAR_COLOR))]
    }
}

#[derive(Clone)]
pub struct ColorDepthMetricsTargets {
    pub rt_color: RenderTexture,
    pub rt_depth: RenderTexture,
    pub metrics: PackedMetricsTargets,
}

impl ColorDepthMetricsTargets {
    pub fn new(device: &wgpu::Device, node_name: &str) -> Self {
        Self {
            rt_color: RenderTexture::empty_color(
                device,
                Some(format!("{}{}", node_name, " rt_color (placeholder)").as_str()),
            ),
            rt_depth: RenderTexture::empty_depth(
                device,
                Some(format!("{}{}", node_name, " rt_depth (placeholder)").as_str()),
            ),
            metrics: PackedMetricsTargets::new(device, node_name),
        }
    }

    pub fn color_attachments<'a>(
        &'a self,
        screen: Option<&'a RenderTexture>,
    ) -> [Option<RenderPassColorAttachment<'a>>; 1] {
        [screen
            .unwrap_or(&self.rt_color)
            .to_color_attachment(Some(CLEAR_COLOR))]
    }

    pub fn depth_attachment(&self) -> Option<RenderPassDepthStencilAttachment<'_>> {
        self.rt_depth.to_depth_attachment(Some(1.0))
    }
}

#[derive(Default)]
pub enum Slot {
    #[default]
    Empty,
    Color {
        color_source: Texture,
        color_target: RenderTexture,
    },
    ColorDepth {
        color_source: Texture,
        color_target: RenderTexture,
        depth_source: Texture,
        depth_target: RenderTexture,
    },
    ColorMetrics {
        color_source: Texture,
        color_target: RenderTexture,
        metrics_a_source: Texture,
        metrics_a_target: RenderTexture,
        metrics_b_source: Texture,
        metrics_b_target: RenderTexture,
        metrics_c_source: Texture,
        metrics_c_target: RenderTexture,
        metrics_d_source: Texture,
        metrics_d_target: RenderTexture,
    },
    ColorDepthMetrics {
        color_source: Texture,
        color_target: RenderTexture,
        depth_source: Texture,
        depth_target: RenderTexture,
        metrics_a_source: Texture,
        metrics_a_target: RenderTexture,
        metrics_b_source: Texture,
        metrics_b_target: RenderTexture,
        metrics_c_source: Texture,
        metrics_c_target: RenderTexture,
        metrics_d_source: Texture,
        metrics_d_target: RenderTexture,
    },
}

pub struct NodeSlots {
    input: Slot,
    output: Slot,
}

impl NodeSlots {
    pub fn new() -> Self {
        Self {
            input: Slot::default(),
            output: Slot::default(),
        }
    }

    pub fn new_io(input: Slot, output: Slot) -> Self {
        Self { input, output }
    }

    pub fn take_input(&mut self) -> Slot {
        std::mem::take(&mut self.input)
    }

    pub fn take_output(&mut self) -> Slot {
        std::mem::take(&mut self.output)
    }

    pub fn to_passthrough(self) -> Self {
        Self {
            input: Slot::Empty,
            output: self.input,
        }
    }

    pub fn to_color_input(self, _context: &RenderContext) -> Self {
        match self.input {
            Slot::Empty => panic!("Input expected"),
            Slot::Color { .. } | Slot::ColorMetrics { .. } => self,
            Slot::ColorDepth {
                color_source,
                color_target,
                ..
            }
            | Slot::ColorDepthMetrics {
                color_source,
                color_target,
                ..
            } => Self {
                input: Slot::Color {
                    color_source,
                    color_target,
                },
                output: self.output,
            },
        }
    }

    pub fn to_color_depth_input(self, _context: &RenderContext) -> Self {
        match self.input {
            Slot::ColorDepth { .. } | Slot::ColorDepthMetrics { .. } => self,
            Slot::Empty | Slot::Color { .. } | Slot::ColorMetrics { .. } => {
                panic!("ColorDepth input expected")
            }
        }
    }

    pub fn to_color_metrics_input(self, _context: &RenderContext) -> Self {
        match self.input {
            Slot::Empty => panic!("Input expected"),
            Slot::Color { .. } | Slot::ColorMetrics { .. } => self,
            Slot::ColorDepthMetrics {
                color_source,
                color_target,
                metrics_a_source,
                metrics_a_target,
                metrics_b_source,
                metrics_b_target,
                metrics_c_source,
                metrics_c_target,
                metrics_d_source,
                metrics_d_target,
                ..
            } => Self {
                input: Slot::ColorMetrics {
                    color_source,
                    color_target,
                    metrics_a_source,
                    metrics_a_target,
                    metrics_b_source,
                    metrics_b_target,
                    metrics_c_source,
                    metrics_c_target,
                    metrics_d_source,
                    metrics_d_target,
                },
                output: self.output,
            },
            Slot::ColorDepth {
                color_source,
                color_target,
                ..
            } => Self {
                input: Slot::Color {
                    color_source,
                    color_target,
                },
                output: self.output,
            },
        }
    }

    pub fn to_color_depth_metrics_input(self, _context: &RenderContext) -> Self {
        match self.input {
            Slot::ColorDepth { .. } | Slot::ColorDepthMetrics { .. } => self,
            Slot::Empty | Slot::Color { .. } | Slot::ColorMetrics { .. } => {
                panic!("ColorDepthMetrics input expected")
            }
        }
    }

    pub fn to_color_output(self, context: &RenderContext, node_name: &str) -> Self {
        match self.output {
            Slot::Empty => {
                let (width, height) = self.input_dimensions();
                let device = context.device();
                let color_target = RenderTexture::create_color(
                    device,
                    width,
                    height,
                    Some(format!("{}{}", node_name, " to_color_output color").as_str()),
                );
                Self {
                    input: self.input,
                    output: Slot::Color {
                        color_source: color_target.as_texture(),
                        color_target,
                    },
                }
            }
            Slot::Color { .. } => self,
            Slot::ColorDepth {
                color_source,
                color_target,
                ..
            }
            | Slot::ColorMetrics {
                color_source,
                color_target,
                ..
            }
            | Slot::ColorDepthMetrics {
                color_source,
                color_target,
                ..
            } => Self {
                input: self.input,
                output: Slot::Color {
                    color_source,
                    color_target,
                },
            },
        }
    }

    pub fn to_color_depth_output(self, context: &RenderContext, node_name: &str) -> Self {
        match self.output {
            Slot::Empty => {
                let (width, height) = self.input_dimensions();
                let device = context.device();
                let color_target = RenderTexture::create_color(
                    device,
                    width,
                    height,
                    Some(format!("{}{}", node_name, " to_color_depth_output color").as_str()),
                );
                let depth_target = RenderTexture::create_depth(
                    device,
                    width,
                    height,
                    Some(format!("{}{}", node_name, " to_color_depth_output depth").as_str()),
                );
                Self {
                    input: self.input,
                    output: Slot::ColorDepth {
                        color_source: color_target.as_texture(),
                        color_target,
                        depth_source: depth_target.as_texture(),
                        depth_target,
                    },
                }
            }
            Slot::ColorDepth { .. } => self,
            Slot::Color {
                color_source,
                color_target,
            }
            | Slot::ColorMetrics {
                color_source,
                color_target,
                ..
            }
            | Slot::ColorDepthMetrics {
                color_source,
                color_target,
                ..
            } => {
                let device = context.device();
                let depth_target = RenderTexture::create_depth(
                    device,
                    color_target.width,
                    color_target.height,
                    Some(format!("{}{}", node_name, " to_color_depth_output depth").as_str()),
                );
                Self {
                    input: self.input,
                    output: Slot::ColorDepth {
                        color_source,
                        color_target,
                        depth_source: depth_target.as_texture(),
                        depth_target,
                    },
                }
            }
        }
    }

    pub fn to_color_metrics_output(self, context: &RenderContext, node_name: &str) -> Self {
        match self.output {
            Slot::ColorMetrics { .. } => self,
            Slot::ColorDepthMetrics {
                color_source,
                color_target,
                metrics_a_source,
                metrics_a_target,
                metrics_b_source,
                metrics_b_target,
                metrics_c_source,
                metrics_c_target,
                metrics_d_source,
                metrics_d_target,
                ..
            } => Self {
                input: self.input,
                output: Slot::ColorMetrics {
                    color_source,
                    color_target,
                    metrics_a_source,
                    metrics_a_target,
                    metrics_b_source,
                    metrics_b_target,
                    metrics_c_source,
                    metrics_c_target,
                    metrics_d_source,
                    metrics_d_target,
                },
            },
            _ => {
                let (width, height) = self.input_dimensions();
                let device = context.device();
                let color_target = RenderTexture::create_color(
                    device,
                    width,
                    height,
                    Some(format!("{}{}", node_name, " to_color_metrics_output color").as_str()),
                );
                let metrics = create_metrics_targets(
                    device,
                    width,
                    height,
                    node_name,
                    "to_color_metrics_output",
                );
                Self {
                    input: self.input,
                    output: Slot::ColorMetrics {
                        color_source: color_target.as_texture(),
                        color_target,
                        metrics_a_source: metrics.rt_metrics_a.as_texture(),
                        metrics_a_target: metrics.rt_metrics_a,
                        metrics_b_source: metrics.rt_metrics_b.as_texture(),
                        metrics_b_target: metrics.rt_metrics_b,
                        metrics_c_source: metrics.rt_metrics_c.as_texture(),
                        metrics_c_target: metrics.rt_metrics_c,
                        metrics_d_source: metrics.rt_metrics_d.as_texture(),
                        metrics_d_target: metrics.rt_metrics_d,
                    },
                }
            }
        }
    }

    pub fn to_color_depth_metrics_output(self, context: &RenderContext, node_name: &str) -> Self {
        match self.output {
            Slot::ColorDepthMetrics { .. } => self,
            _ => {
                let (width, height) = self.input_dimensions();
                let device = context.device();
                let color_target = RenderTexture::create_color(
                    device,
                    width,
                    height,
                    Some(
                        format!("{}{}", node_name, " to_color_depth_metrics_output color").as_str(),
                    ),
                );
                let depth_target = RenderTexture::create_depth(
                    device,
                    width,
                    height,
                    Some(
                        format!("{}{}", node_name, " to_color_depth_metrics_output depth").as_str(),
                    ),
                );
                let metrics = create_metrics_targets(
                    device,
                    width,
                    height,
                    node_name,
                    "to_color_depth_metrics_output",
                );
                Self {
                    input: self.input,
                    output: Slot::ColorDepthMetrics {
                        color_source: color_target.as_texture(),
                        color_target,
                        depth_source: depth_target.as_texture(),
                        depth_target,
                        metrics_a_source: metrics.rt_metrics_a.as_texture(),
                        metrics_a_target: metrics.rt_metrics_a,
                        metrics_b_source: metrics.rt_metrics_b.as_texture(),
                        metrics_b_target: metrics.rt_metrics_b,
                        metrics_c_source: metrics.rt_metrics_c.as_texture(),
                        metrics_c_target: metrics.rt_metrics_c,
                        metrics_d_source: metrics.rt_metrics_d.as_texture(),
                        metrics_d_target: metrics.rt_metrics_d,
                    },
                }
            }
        }
    }

    pub fn emplace_color_output(
        self,
        context: &RenderContext,
        width: u32,
        height: u32,
        node_name: &str,
    ) -> Self {
        let device = context.device();
        let color_target = RenderTexture::create_color(
            device,
            width,
            height,
            Some(format!("{}{}", node_name, " emplace_color_output color").as_str()),
        );
        Self {
            input: self.input,
            output: Slot::Color {
                color_source: color_target.as_texture(),
                color_target,
            },
        }
    }

    pub fn emplace_color_depth_output(
        self,
        context: &RenderContext,
        width: u32,
        height: u32,
        node_name: &str,
    ) -> Self {
        let device = context.device();
        let color_target = RenderTexture::create_color(
            device,
            width,
            height,
            Some(format!("{}{}", node_name, " emplace_color_depth_output color").as_str()),
        );
        let depth_target = RenderTexture::create_depth(
            device,
            width,
            height,
            Some(format!("{}{}", node_name, " emplace_color_depth_output depth").as_str()),
        );
        Self {
            input: self.input,
            output: Slot::ColorDepth {
                color_source: color_target.as_texture(),
                color_target,
                depth_source: depth_target.as_texture(),
                depth_target,
            },
        }
    }

    pub fn as_color_source(&self, device: &wgpu::Device) -> (Texture, BindGroup) {
        match &self.input {
            Slot::Color { color_source, .. }
            | Slot::ColorDepth { color_source, .. }
            | Slot::ColorMetrics { color_source, .. }
            | Slot::ColorDepthMetrics { color_source, .. } => {
                let (_, bind_group) = color_source.create_bind_group(device);
                (color_source.clone(), bind_group)
            }
            Slot::Empty => panic!("Color input expected"),
        }
    }

    pub fn as_color_depth_source(
        &self,
        device: &wgpu::Device,
    ) -> ((Texture, BindGroup), (Texture, BindGroup)) {
        match &self.input {
            Slot::ColorDepth {
                color_source,
                depth_source,
                ..
            }
            | Slot::ColorDepthMetrics {
                color_source,
                depth_source,
                ..
            } => {
                let (_, bind_group_color) = color_source.create_bind_group(device);
                let (_, bind_group_depth) = depth_source.create_bind_group(device);
                (
                    (color_source.clone(), bind_group_color),
                    (depth_source.clone(), bind_group_depth),
                )
            }
            _ => panic!("ColorDepth input expected"),
        }
    }

    pub fn as_all_colors_source(&self, device: &wgpu::Device, queue: &wgpu::Queue) -> BindGroup {
        let metric_placeholders = metric_placeholders(device, queue, "NodeSlots color metrics");
        match &self.input {
            Slot::ColorMetrics {
                color_source,
                metrics_a_source,
                metrics_b_source,
                metrics_c_source,
                metrics_d_source,
                ..
            } => {
                [
                    color_source,
                    metrics_a_source,
                    metrics_b_source,
                    metrics_c_source,
                    metrics_d_source,
                ]
                .create_bind_group(device)
                .1
            }
            Slot::Color { color_source, .. } => {
                [
                    color_source,
                    &metric_placeholders[0],
                    &metric_placeholders[1],
                    &metric_placeholders[2],
                    &metric_placeholders[3],
                ]
                .create_bind_group(device)
                .1
            }
            Slot::ColorDepthMetrics {
                color_source,
                metrics_a_source,
                metrics_b_source,
                metrics_c_source,
                metrics_d_source,
                ..
            } => {
                [
                    color_source,
                    metrics_a_source,
                    metrics_b_source,
                    metrics_c_source,
                    metrics_d_source,
                ]
                .create_bind_group(device)
                .1
            }
            Slot::ColorDepth { color_source, .. } => {
                [
                    color_source,
                    &metric_placeholders[0],
                    &metric_placeholders[1],
                    &metric_placeholders[2],
                    &metric_placeholders[3],
                ]
                .create_bind_group(device)
                .1
            }
            Slot::Empty => panic!("ColorMetrics input expected"),
        }
    }

    pub fn as_all_source(&self, device: &wgpu::Device, queue: &wgpu::Queue) -> BindGroup {
        let metric_placeholders = metric_placeholders(device, queue, "NodeSlots depth metrics");
        match &self.input {
            Slot::ColorDepthMetrics {
                color_source,
                depth_source,
                metrics_a_source,
                metrics_b_source,
                metrics_c_source,
                metrics_d_source,
                ..
            } => {
                [
                    color_source,
                    depth_source,
                    metrics_a_source,
                    metrics_b_source,
                    metrics_c_source,
                    metrics_d_source,
                ]
                .create_bind_group(device)
                .1
            }
            Slot::ColorDepth {
                color_source,
                depth_source,
                ..
            } => {
                [
                    color_source,
                    depth_source,
                    &metric_placeholders[0],
                    &metric_placeholders[1],
                    &metric_placeholders[2],
                    &metric_placeholders[3],
                ]
                .create_bind_group(device)
                .1
            }
            _ => panic!("ColorDepthMetrics input expected"),
        }
    }

    pub fn as_color_target(&self) -> RenderTexture {
        match &self.output {
            Slot::Color { color_target, .. }
            | Slot::ColorDepth { color_target, .. }
            | Slot::ColorMetrics { color_target, .. }
            | Slot::ColorDepthMetrics { color_target, .. } => color_target.clone(),
            Slot::Empty => panic!("Color output expected"),
        }
    }

    pub fn as_color_depth_target(&self) -> (RenderTexture, RenderTexture) {
        match &self.output {
            Slot::ColorDepth {
                color_target,
                depth_target,
                ..
            }
            | Slot::ColorDepthMetrics {
                color_target,
                depth_target,
                ..
            } => (color_target.clone(), depth_target.clone()),
            _ => panic!("ColorDepth output expected"),
        }
    }

    pub fn as_color_targets(&self) -> ColorTargets {
        let rt_color = self.as_color_target();
        ColorTargets {
            rt_color: rt_color.clone(),
            rt_deflection: rt_color.clone(),
            rt_color_change: rt_color.clone(),
            rt_color_uncertainty: rt_color.clone(),
            rt_covariances: rt_color,
        }
    }

    pub fn as_color_depth_targets(&self) -> ColorDepthTargets {
        let (rt_color, rt_depth) = self.as_color_depth_target();
        ColorDepthTargets {
            rt_color: rt_color.clone(),
            rt_depth,
            rt_deflection: rt_color.clone(),
            rt_color_change: rt_color.clone(),
            rt_color_uncertainty: rt_color.clone(),
            rt_covariances: rt_color,
        }
    }

    pub fn as_all_colors_target(&self) -> ColorTargets {
        match &self.output {
            Slot::ColorMetrics {
                color_target,
                metrics_a_target,
                metrics_b_target,
                metrics_c_target,
                metrics_d_target,
                ..
            }
            | Slot::ColorDepthMetrics {
                color_target,
                metrics_a_target,
                metrics_b_target,
                metrics_c_target,
                metrics_d_target,
                ..
            } => ColorTargets {
                rt_color: color_target.clone(),
                rt_deflection: metrics_a_target.clone(),
                rt_color_change: metrics_b_target.clone(),
                rt_color_uncertainty: metrics_c_target.clone(),
                rt_covariances: metrics_d_target.clone(),
            },
            _ => panic!("ColorMetrics output expected"),
        }
    }

    pub fn as_all_target(&self) -> ColorDepthTargets {
        match &self.output {
            Slot::ColorDepthMetrics {
                color_target,
                depth_target,
                metrics_a_target,
                metrics_b_target,
                metrics_c_target,
                metrics_d_target,
                ..
            } => ColorDepthTargets {
                rt_color: color_target.clone(),
                rt_depth: depth_target.clone(),
                rt_deflection: metrics_a_target.clone(),
                rt_color_change: metrics_b_target.clone(),
                rt_color_uncertainty: metrics_c_target.clone(),
                rt_covariances: metrics_d_target.clone(),
            },
            _ => panic!("ColorDepthMetrics output expected"),
        }
    }

    fn input_dimensions(&self) -> (u32, u32) {
        let target = match &self.input {
            Slot::Empty => panic!("Input expected"),
            Slot::Color { color_target, .. }
            | Slot::ColorDepth { color_target, .. }
            | Slot::ColorMetrics { color_target, .. }
            | Slot::ColorDepthMetrics { color_target, .. } => color_target,
        };
        (target.width, target.height)
    }

    fn output_size(&self) -> [u32; 2] {
        let target = match &self.output {
            Slot::Empty => panic!("Output expected"),
            Slot::Color { color_target, .. }
            | Slot::ColorDepth { color_target, .. }
            | Slot::ColorMetrics { color_target, .. }
            | Slot::ColorDepthMetrics { color_target, .. } => color_target,
        };
        [target.width, target.height]
    }

    pub fn output_size_f32(&self) -> [f32; 2] {
        let size = self.output_size();
        [size[0] as f32, size[1] as f32]
    }

    pub fn input_size_f32(&self) -> [f32; 2] {
        let (width, height) = self.input_dimensions();
        [width as f32, height as f32]
    }
}

fn create_metrics_targets(
    device: &wgpu::Device,
    width: u32,
    height: u32,
    node_name: &str,
    suffix: &str,
) -> PackedMetricsTargets {
    PackedMetricsTargets {
        rt_metrics_a: RenderTexture::create_metrics(
            device,
            width,
            height,
            Some(format!("{} {} metrics_a", node_name, suffix).as_str()),
        ),
        rt_metrics_b: RenderTexture::create_metrics(
            device,
            width,
            height,
            Some(format!("{} {} metrics_b", node_name, suffix).as_str()),
        ),
        rt_metrics_c: RenderTexture::create_metrics(
            device,
            width,
            height,
            Some(format!("{} {} metrics_c", node_name, suffix).as_str()),
        ),
        rt_metrics_d: RenderTexture::create_metrics(
            device,
            width,
            height,
            Some(format!("{} {} metrics_d", node_name, suffix).as_str()),
        ),
    }
}

fn metric_placeholders(device: &wgpu::Device, queue: &wgpu::Queue, label: &str) -> [Texture; 4] {
    [
        placeholder_metrics_texture(device, queue, Some(format!("{} a", label).as_str())).unwrap(),
        placeholder_metrics_texture(device, queue, Some(format!("{} b", label).as_str())).unwrap(),
        placeholder_metrics_texture(device, queue, Some(format!("{} c", label).as_str())).unwrap(),
        placeholder_metrics_texture(device, queue, Some(format!("{} d", label).as_str())).unwrap(),
    ]
}
