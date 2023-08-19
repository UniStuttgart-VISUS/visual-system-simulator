use wgpu::{BindGroup, RenderPassColorAttachment, RenderPassDepthStencilAttachment};

use super::*;

//TODO: we might need to select this at runtime.
pub static COLOR_FORMAT: wgpu::TextureFormat = if cfg!(target_arch = "wasm32") {
    wgpu::TextureFormat::Rgba8Unorm
} else if cfg!(target_os = "android") {
    wgpu::TextureFormat::Rgba8UnormSrgb
} else {
    wgpu::TextureFormat::Bgra8Unorm
};
pub static HIGHP_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba32Float;
pub static DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;

pub static CLEAR_COLOR: wgpu::Color = wgpu::Color {
    r: 0.5,
    g: 0.5,
    b: 0.5,
    a: 1.0,
};

pub struct ColorDepthTargets {
    rt_color: RenderTexture,
    rt_depth: RenderTexture,
    rt_deflection: RenderTexture,
    rt_color_change: RenderTexture,
    rt_color_uncertainty: RenderTexture,
    rt_covariances: RenderTexture,
}

impl ColorDepthTargets {
    pub fn new(device: &wgpu::Device, node_name: &str) -> Self {
        let rt_color = RenderTexture::empty_color(
            device,
            Some(format!("{}{}", node_name, " rt_color (placeholder)").as_str()),
        );
        let rt_depth = RenderTexture::empty_depth(
            device,
            Some(format!("{}{}", node_name, " rt_depth (placeholder)").as_str()),
        );
        let rt_deflection = RenderTexture::empty_highp(
            device,
            Some(format!("{}{}", node_name, " rt_deflection (placeholder)").as_str()),
        );
        let rt_color_change = RenderTexture::empty_highp(
            device,
            Some(format!("{}{}", node_name, " rt_color_change (placeholder)").as_str()),
        );
        let rt_color_uncertainty = RenderTexture::empty_highp(
            device,
            Some(format!("{}{}", node_name, " rt_color_uncertainty (placeholder)").as_str()),
        );
        let rt_covariances = RenderTexture::empty_highp(
            device,
            Some(format!("{}{}", node_name, " rt_covariances (placeholder)").as_str()),
        );

        Self {
            rt_color,
            rt_depth,
            rt_deflection,
            rt_color_change,
            rt_color_uncertainty,
            rt_covariances,
        }
    }

    pub fn color_attachments<'a>(
        &'a self,
        screen: Option<&'a RenderTexture>,
    ) -> [Option<RenderPassColorAttachment>; 5] {
        [
            screen
                .unwrap_or(&self.rt_color)
                .to_color_attachment(Some(CLEAR_COLOR)),
            self.rt_deflection.to_color_attachment(Some(CLEAR_COLOR)),
            self.rt_color_change.to_color_attachment(Some(CLEAR_COLOR)),
            self.rt_color_uncertainty
                .to_color_attachment(Some(CLEAR_COLOR)),
            self.rt_covariances.to_color_attachment(Some(CLEAR_COLOR)),
        ]
    }

    pub fn depth_attachment(&self) -> Option<RenderPassDepthStencilAttachment> {
        self.rt_depth.to_depth_attachment(Some(1.0))
    }
}

pub struct ColorTargets {
    pub rt_color: RenderTexture,
    pub rt_deflection: RenderTexture,
    pub rt_color_change: RenderTexture,
    pub rt_color_uncertainty: RenderTexture,
    pub rt_covariances: RenderTexture,
}

impl ColorTargets {
    pub fn new(device: &wgpu::Device, node_name: &str) -> Self {
        let rt_color = RenderTexture::empty_color(
            device,
            Some(format!("{}{}", node_name, " rt_color (placeholder)").as_str()),
        );
        let rt_deflection = RenderTexture::empty_highp(
            device,
            Some(format!("{}{}", node_name, " rt_deflection (placeholder)").as_str()),
        );
        let rt_color_change = RenderTexture::empty_highp(
            device,
            Some(format!("{}{}", node_name, " rt_color_change (placeholder)").as_str()),
        );
        let rt_color_uncertainty = RenderTexture::empty_highp(
            device,
            Some(format!("{}{}", node_name, " rt_color_uncertainty (placeholder)").as_str()),
        );
        let rt_covariances = RenderTexture::empty_highp(
            device,
            Some(format!("{}{}", node_name, " rt_covariances (placeholder)").as_str()),
        );

        Self {
            rt_color,
            rt_deflection,
            rt_color_change,
            rt_color_uncertainty,
            rt_covariances,
        }
    }

    pub fn color_attachments<'a>(
        &'a self,
        screen: Option<&'a RenderTexture>,
    ) -> [Option<RenderPassColorAttachment>; 5] {
        [
            screen
                .unwrap_or(&self.rt_color)
                .to_color_attachment(Some(CLEAR_COLOR)),
            self.rt_deflection.to_color_attachment(Some(CLEAR_COLOR)),
            self.rt_color_change.to_color_attachment(Some(CLEAR_COLOR)),
            self.rt_color_uncertainty
                .to_color_attachment(Some(CLEAR_COLOR)),
            self.rt_covariances.to_color_attachment(Some(CLEAR_COLOR)),
        ]
    }
}

#[derive(Default)]
pub enum Slot {
    #[default]
    Empty,
    Rgb {
        // TODO remove, but all shaders need to be adjusted to deal with depth appropriately
        color_source: Texture,
        color_target: RenderTexture,
        deflection_source: Texture,
        deflection_target: RenderTexture,
        color_change_source: Texture,
        color_change_target: RenderTexture,
        color_uncertainty_source: Texture,
        color_uncertainty_target: RenderTexture,
        covariances_source: Texture,
        covariances_target: RenderTexture,
    },
    RgbDepth {
        color_source: Texture,
        color_target: RenderTexture,
        depth_source: Texture,
        depth_target: RenderTexture,
        deflection_source: Texture,
        deflection_target: RenderTexture,
        color_change_source: Texture,
        color_change_target: RenderTexture,
        color_uncertainty_source: Texture,
        color_uncertainty_target: RenderTexture,
        covariances_source: Texture,
        covariances_target: RenderTexture,
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

    pub fn to_color_input(self, _surface: &Surface) -> Self {
        match self.input {
            Slot::Empty => {
                panic!("Input expected");
            }
            Slot::Rgb { .. } => self,
            Slot::RgbDepth {
                color_source,
                color_target,
                deflection_source,
                deflection_target,
                color_change_source,
                color_change_target,
                color_uncertainty_source,
                color_uncertainty_target,
                covariances_source,
                covariances_target,
                ..
            } => Self {
                input: Slot::Rgb {
                    color_source,
                    color_target,
                    deflection_source,
                    deflection_target,
                    color_change_source,
                    color_change_target,
                    color_uncertainty_source,
                    color_uncertainty_target,
                    covariances_source,
                    covariances_target,
                },
                output: self.output,
            },
        }
    }

    pub fn to_color_depth_input(self, _surface: &Surface) -> Self {
        match self.input {
            Slot::Empty => {
                panic!("Input expected");
            }
            Slot::Rgb { .. } => {
                panic!("RGB input cannot be extended with depth");
            }
            Slot::RgbDepth { .. } => self,
        }
    }

    pub fn to_color_output(self, surface: &Surface, node_name: &str) -> Self {
        match self.output {
            Slot::Empty => {
                // Guess output size, based on input.
                let (width, height) = match &self.input {
                    Slot::Empty => {
                        panic!("Input expected");
                    }
                    Slot::Rgb { color_target, .. } => (color_target.width, color_target.height),
                    Slot::RgbDepth { color_target, .. } => {
                        (color_target.width, color_target.height)
                    }
                };
                let device = surface.device();
                let color_target = RenderTexture::create_color(
                    device,
                    width,
                    height,
                    Some(format!("{}{}", node_name, " to_color_output color").as_str()),
                );
                let deflection_target = RenderTexture::create_highp(
                    device,
                    width,
                    height,
                    Some(format!("{}{}", node_name, " to_color_output deflection").as_str()),
                );
                let color_change_target = RenderTexture::create_highp(
                    device,
                    width,
                    height,
                    Some(format!("{}{}", node_name, " to_color_output color_change").as_str()),
                );
                let color_uncertainty_target = RenderTexture::create_highp(
                    device,
                    width,
                    height,
                    Some(format!("{}{}", node_name, " to_color_output color_uncertainty").as_str()),
                );
                let covariances_target = RenderTexture::create_highp(
                    device,
                    width,
                    height,
                    Some(format!("{}{}", node_name, " to_color_output covariances").as_str()),
                );

                Self {
                    input: self.input,
                    output: Slot::Rgb {
                        color_source: color_target.as_texture(),
                        color_target,
                        deflection_source: deflection_target.as_texture(),
                        deflection_target,
                        color_change_source: color_change_target.as_texture(),
                        color_change_target,
                        color_uncertainty_source: color_uncertainty_target.as_texture(),
                        color_uncertainty_target,
                        covariances_source: covariances_target.as_texture(),
                        covariances_target,
                    },
                }
            }
            Slot::Rgb { .. } => self,
            Slot::RgbDepth {
                color_source,
                color_target,
                deflection_source,
                deflection_target,
                color_change_source,
                color_change_target,
                color_uncertainty_source,
                color_uncertainty_target,
                covariances_source,
                covariances_target,
                ..
            } => Self {
                input: self.input,
                output: Slot::Rgb {
                    color_source,
                    color_target,
                    deflection_source,
                    deflection_target,
                    color_change_source,
                    color_change_target,
                    color_uncertainty_source,
                    color_uncertainty_target,
                    covariances_source,
                    covariances_target,
                },
            },
        }
    }

    pub fn to_color_depth_output(self, surface: &Surface, node_name: &str) -> Self {
        match self.output {
            Slot::Empty => {
                // Guess output size, based on input.
                let (width, height) = match &self.input {
                    Slot::Empty => {
                        panic!("Input expected");
                    }
                    Slot::Rgb { color_target, .. } => (color_target.width, color_target.height),
                    Slot::RgbDepth { color_target, .. } => {
                        (color_target.width, color_target.height)
                    }
                };
                let device = surface.device();
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
                let deflection_target = RenderTexture::create_highp(
                    device,
                    width,
                    height,
                    Some(format!("{}{}", node_name, " to_color_depth_output deflection").as_str()),
                );
                let color_change_target = RenderTexture::create_highp(
                    device,
                    width,
                    height,
                    Some(
                        format!("{}{}", node_name, " to_color_depth_output color_change").as_str(),
                    ),
                );
                let color_uncertainty_target = RenderTexture::create_highp(
                    device,
                    width,
                    height,
                    Some(
                        format!(
                            "{}{}",
                            node_name, " to_color_depth_output color_uncertainty"
                        )
                        .as_str(),
                    ),
                );
                let covariances_target = RenderTexture::create_highp(
                    device,
                    width,
                    height,
                    Some(format!("{}{}", node_name, " to_color_depth_output covariances").as_str()),
                );

                Self {
                    input: self.input,
                    output: Slot::RgbDepth {
                        color_source: color_target.as_texture(),
                        color_target,
                        depth_source: depth_target.as_texture(),
                        depth_target,
                        deflection_source: deflection_target.as_texture(),
                        deflection_target,
                        color_change_source: color_change_target.as_texture(),
                        color_change_target,
                        color_uncertainty_source: color_uncertainty_target.as_texture(),
                        color_uncertainty_target,
                        covariances_source: covariances_target.as_texture(),
                        covariances_target,
                    },
                }
            }
            Slot::Rgb {
                color_source,
                color_target,
                deflection_source,
                deflection_target,
                color_change_source,
                color_change_target,
                color_uncertainty_source,
                color_uncertainty_target,
                covariances_source,
                covariances_target,
                ..
            } => {
                // Guess missing depth, based on color.
                let device = surface.device();
                let depth_target = RenderTexture::create_depth(
                    device,
                    color_target.width,
                    color_target.height,
                    Some(format!("{}{}", node_name, " to_color_depth_output depth").as_str()),
                );
                Self {
                    input: self.input,
                    output: Slot::RgbDepth {
                        color_source,
                        color_target,
                        depth_source: depth_target.as_texture(),
                        depth_target,
                        deflection_source,
                        deflection_target,
                        color_change_source,
                        color_change_target,
                        color_uncertainty_source,
                        color_uncertainty_target,
                        covariances_source,
                        covariances_target,
                    },
                }
            }
            Slot::RgbDepth { .. } => self,
        }
    }

    pub fn emplace_color_output(
        self,
        surface: &Surface,
        width: u32,
        height: u32,
        node_name: &str,
    ) -> Self {
        let device = surface.device();
        let color_target = RenderTexture::create_color(
            device,
            width,
            height,
            Some(format!("{}{}", node_name, " emplace_color_output color").as_str()),
        );
        let deflection_target = RenderTexture::create_highp(
            device,
            width,
            height,
            Some(format!("{}{}", node_name, " emplace_color_output deflection").as_str()),
        );
        let color_change_target = RenderTexture::create_highp(
            device,
            width,
            height,
            Some(format!("{}{}", node_name, " emplace_color_output color_change").as_str()),
        );
        let color_uncertainty_target = RenderTexture::create_highp(
            device,
            width,
            height,
            Some(format!("{}{}", node_name, " emplace_color_output color_uncertainty").as_str()),
        );
        let covariances_target = RenderTexture::create_highp(
            device,
            width,
            height,
            Some(format!("{}{}", node_name, " emplace_color_output covariances").as_str()),
        );

        Self {
            input: self.input,
            output: Slot::Rgb {
                color_source: color_target.as_texture(),
                color_target,
                deflection_source: deflection_target.as_texture(),
                deflection_target,
                color_change_source: color_change_target.as_texture(),
                color_change_target,
                color_uncertainty_source: color_uncertainty_target.as_texture(),
                color_uncertainty_target,
                covariances_source: covariances_target.as_texture(),
                covariances_target,
            },
        }
    }

    pub fn emplace_color_depth_output(
        self,
        surface: &Surface,
        width: u32,
        height: u32,
        node_name: &str,
    ) -> Self {
        let device = surface.device();
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
        let deflection_target = RenderTexture::create_highp(
            device,
            width,
            height,
            Some(format!("{}{}", node_name, " emplace_color_depth_output deflection").as_str()),
        );
        let color_change_target = RenderTexture::create_highp(
            device,
            width,
            height,
            Some(
                format!(
                    "{}{}",
                    node_name, " emplace_color_depth_output color_change"
                )
                .as_str(),
            ),
        );
        let color_uncertainty_target = RenderTexture::create_highp(
            device,
            width,
            height,
            Some(
                format!(
                    "{}{}",
                    node_name, " emplace_color_depth_output color_uncertainty"
                )
                .as_str(),
            ),
        );
        let covariances_target = RenderTexture::create_highp(
            device,
            width,
            height,
            Some(format!("{}{}", node_name, " emplace_color_depth_output covariances").as_str()),
        );

        Self {
            input: self.input,
            output: Slot::RgbDepth {
                color_source: color_target.as_texture(),
                color_target,
                depth_source: depth_target.as_texture(),
                depth_target,
                deflection_source: deflection_target.as_texture(),
                deflection_target,
                color_change_source: color_change_target.as_texture(),
                color_change_target,
                color_uncertainty_source: color_uncertainty_target.as_texture(),
                color_uncertainty_target,
                covariances_source: covariances_target.as_texture(),
                covariances_target,
            },
        }
    }

    pub fn as_color_source(&self, device: &wgpu::Device) -> (Texture, BindGroup) {
        match &self.input {
            Slot::Empty | Slot::RgbDepth { .. } => {
                panic!("RGB input expected");
            }
            Slot::Rgb { color_source, .. } => {
                let (_, bind_group) = color_source.create_bind_group(device);
                (color_source.clone(), bind_group)
            }
        }
    }

    pub fn as_color_depth_source(
        &self,
        device: &wgpu::Device,
    ) -> ((Texture, BindGroup), (Texture, BindGroup)) {
        match &self.input {
            Slot::Empty | Slot::Rgb { .. } => {
                panic!("RGBD input expected");
            }
            Slot::RgbDepth {
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
        }
    }

    pub fn as_deflection_source(&self, device: &wgpu::Device) -> (Texture, BindGroup) {
        match &self.input {
            Slot::Empty | Slot::RgbDepth { .. } => {
                panic!("RGB input expected");
            }
            Slot::Rgb {
                deflection_source, ..
            } => {
                let (_, bind_group) = deflection_source.create_bind_group(device);
                (deflection_source.clone(), bind_group)
            }
        }
    }

    pub fn as_color_change_source(&self, device: &wgpu::Device) -> (Texture, BindGroup) {
        match &self.input {
            Slot::Empty | Slot::RgbDepth { .. } => {
                panic!("RGB input expected");
            }
            Slot::Rgb {
                color_change_source,
                ..
            } => {
                let (_, bind_group) = color_change_source.create_bind_group(device);
                (color_change_source.clone(), bind_group)
            }
        }
    }

    pub fn as_color_uncertainty_source(&self, device: &wgpu::Device) -> (Texture, BindGroup) {
        match &self.input {
            Slot::Empty | Slot::RgbDepth { .. } => {
                panic!("RGB input expected");
            }
            Slot::Rgb {
                color_uncertainty_source,
                ..
            } => {
                let (_, bind_group) = color_uncertainty_source.create_bind_group(device);
                (color_uncertainty_source.clone(), bind_group)
            }
        }
    }

    pub fn as_covariances_source(&self, device: &wgpu::Device) -> (Texture, BindGroup) {
        match &self.input {
            Slot::Empty | Slot::RgbDepth { .. } => {
                panic!("RGB input expected");
            }
            Slot::Rgb {
                covariances_source, ..
            } => {
                let (_, bind_group) = covariances_source.create_bind_group(device);
                (covariances_source.clone(), bind_group)
            }
        }
    }

    pub fn as_all_colors_source(&self, device: &wgpu::Device) -> BindGroup {
        match &self.input {
            Slot::Empty | Slot::RgbDepth { .. } => {
                panic!("RGB input expected");
            }
            Slot::Rgb {
                color_source,
                deflection_source,
                color_change_source,
                color_uncertainty_source,
                covariances_source,
                ..
            } => {
                [
                    color_source,
                    deflection_source,
                    color_change_source,
                    color_uncertainty_source,
                    covariances_source,
                ]
                .create_bind_group(device)
                .1
            }
        }
    }

    pub fn as_all_source(&self, device: &wgpu::Device) -> BindGroup {
        match &self.input {
            Slot::Empty | Slot::Rgb { .. } => {
                panic!("RGB Depth input expected");
            }
            Slot::RgbDepth {
                color_source,
                depth_source,
                deflection_source,
                color_change_source,
                color_uncertainty_source,
                covariances_source,
                ..
            } => {
                [
                    color_source,
                    depth_source,
                    deflection_source,
                    color_change_source,
                    color_uncertainty_source,
                    covariances_source,
                ]
                .create_bind_group(device)
                .1
            }
        }
    }

    pub fn as_color_target(&self) -> RenderTexture {
        match &self.output {
            Slot::Empty | Slot::RgbDepth { .. } => {
                panic!("RGB output expected");
            }
            Slot::Rgb { color_target, .. } => color_target.clone(),
        }
    }

    pub fn as_color_depth_target(&self) -> (RenderTexture, RenderTexture) {
        match &self.output {
            Slot::Empty | Slot::Rgb { .. } => {
                panic!("RGBD output expected");
            }
            Slot::RgbDepth {
                color_target,
                depth_target,
                ..
            } => (color_target.clone(), depth_target.clone()),
        }
    }

    pub fn as_deflection_target(&self) -> RenderTexture {
        match &self.output {
            Slot::Empty | Slot::RgbDepth { .. } => {
                panic!("RGB output expected");
            }
            Slot::Rgb {
                deflection_target, ..
            } => deflection_target.clone(),
        }
    }

    pub fn as_color_change_target(&self) -> RenderTexture {
        match &self.output {
            Slot::Empty | Slot::RgbDepth { .. } => {
                panic!("RGB output expected");
            }
            Slot::Rgb {
                color_change_target,
                ..
            } => color_change_target.clone(),
        }
    }

    pub fn as_color_uncertainty_target(&self) -> RenderTexture {
        match &self.output {
            Slot::Empty | Slot::RgbDepth { .. } => {
                panic!("RGB output expected");
            }
            Slot::Rgb {
                color_uncertainty_target,
                ..
            } => color_uncertainty_target.clone(),
        }
    }

    pub fn as_covariances_target(&self) -> RenderTexture {
        match &self.output {
            Slot::Empty | Slot::RgbDepth { .. } => {
                panic!("RGB output expected");
            }
            Slot::Rgb {
                covariances_target, ..
            } => covariances_target.clone(),
        }
    }

    pub fn as_all_target(&self) -> ColorDepthTargets {
        match &self.output {
            Slot::Empty | Slot::Rgb { .. } => {
                panic!("RGBD output expected");
            }
            Slot::RgbDepth {
                color_target,
                depth_target,
                deflection_target,
                color_change_target,
                color_uncertainty_target,
                covariances_target,
                ..
            } => ColorDepthTargets {
                rt_color: color_target.clone(),
                rt_depth: depth_target.clone(),
                rt_deflection: deflection_target.clone(),
                rt_color_change: color_change_target.clone(),
                rt_color_uncertainty: color_uncertainty_target.clone(),
                rt_covariances: covariances_target.clone(),
            },
        }
    }

    pub fn as_all_colors_target(&self) -> ColorTargets {
        match &self.output {
            Slot::Empty | Slot::RgbDepth { .. } => {
                panic!("RGB output expected");
            }
            Slot::Rgb {
                color_target,
                deflection_target,
                color_change_target,
                color_uncertainty_target,
                covariances_target,
                ..
            } => ColorTargets {
                rt_color: color_target.clone(),
                rt_deflection: deflection_target.clone(),
                rt_color_change: color_change_target.clone(),
                rt_color_uncertainty: color_uncertainty_target.clone(),
                rt_covariances: covariances_target.clone(),
            },
        }
    }

    fn output_size(&self) -> [u32; 2] {
        let target = match &self.output {
            Slot::Empty => {
                panic!("Output expected");
            }
            Slot::Rgb { color_target, .. } => color_target,
            Slot::RgbDepth { color_target, .. } => color_target,
        };

        [target.width, target.height]
    }

    pub fn output_size_f32(&self) -> [f32; 2] {
        let size = self.output_size();
        [size[0] as f32, size[1] as f32]
    }

    pub fn input_size_f32(&self) -> [f32; 2] {
        let target = match &self.input {
            Slot::Empty => {
                panic!("Output expected");
            }
            Slot::Rgb { color_target, .. } => color_target,
            Slot::RgbDepth { color_target, .. } => color_target,
        };

        [target.width as f32, target.height as f32]
    }
}
