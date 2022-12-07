use eframe::egui::color;
use env_logger::fmt::Color;
use wgpu::{BindGroup, RenderPassColorAttachment, RenderPassDepthStencilAttachment};

use super::*;
use std::cell::RefCell;

// pub static ColorFormat: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm; // old color format
pub static COLOR_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Bgra8Unorm;
pub static HIGHP_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba32Float;
pub static DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;

pub struct ColorDepthTargets{
    rt_color: RenderTexture,
    rt_depth: RenderTexture,
    rt_deflection: RenderTexture,
    rt_color_change: RenderTexture,
    rt_color_uncertainty: RenderTexture,
    rt_covariances: RenderTexture,
}

impl ColorDepthTargets{
    pub fn new(device: &wgpu::Device, node_name: &str) -> Self{
        let rt_color = placeholder_color_rt(&device, Some(format!("{}{}", node_name, " rt_color (placeholder)").as_str()));
        let rt_depth = placeholder_depth_rt(&device, Some(format!("{}{}", node_name, " rt_depth (placeholder)").as_str()));
        let rt_deflection = placeholder_highp_rt(&device, Some(format!("{}{}", node_name, " rt_deflection (placeholder)").as_str()));
        let rt_color_change = placeholder_highp_rt(&device, Some(format!("{}{}", node_name, " rt_color_change (placeholder)").as_str()));
        let rt_color_uncertainty = placeholder_highp_rt(&device, Some(format!("{}{}", node_name, " rt_color_uncertainty (placeholder)").as_str()));
        let rt_covariances = placeholder_highp_rt(&device, Some(format!("{}{}", node_name, " rt_covariances (placeholder)").as_str()));

        Self{
            rt_color,
            rt_depth,
            rt_deflection,
            rt_color_change,
            rt_color_uncertainty,
            rt_covariances,
        }
    }

    pub fn color_attachments<'a>(&'a self, screen: Option<&'a RenderTexture>) -> [Option<RenderPassColorAttachment>; 5]{
        [
            screen.unwrap_or(&self.rt_color).to_color_attachment(),
            self.rt_deflection.to_color_attachment(),
            self.rt_color_change.to_color_attachment(),
            self.rt_color_uncertainty.to_color_attachment(),
            self.rt_covariances.to_color_attachment(),
        ]
    }

    pub fn depth_attachment(&self) -> Option<RenderPassDepthStencilAttachment>{
        self.rt_depth.to_depth_attachment()
    }
}

pub struct ColorTargets{
    rt_color: RenderTexture,
    rt_deflection: RenderTexture,
    rt_color_change: RenderTexture,
    rt_color_uncertainty: RenderTexture,
    rt_covariances: RenderTexture,
}

impl ColorTargets{
    pub fn new(device: &wgpu::Device, node_name: &str) -> Self{
        let rt_color = placeholder_color_rt(&device, Some(format!("{}{}", node_name, " rt_color (placeholder)").as_str()));
        let rt_deflection = placeholder_highp_rt(&device, Some(format!("{}{}", node_name, " rt_deflection (placeholder)").as_str()));
        let rt_color_change = placeholder_highp_rt(&device, Some(format!("{}{}", node_name, " rt_color_change (placeholder)").as_str()));
        let rt_color_uncertainty = placeholder_highp_rt(&device, Some(format!("{}{}", node_name, " rt_color_uncertainty (placeholder)").as_str()));
        let rt_covariances = placeholder_highp_rt(&device, Some(format!("{}{}", node_name, " rt_covariances (placeholder)").as_str()));

        Self{
            rt_color,
            rt_deflection,
            rt_color_change,
            rt_color_uncertainty,
            rt_covariances,
        }
    }
    
    pub fn color_attachments<'a>(&'a self, screen: Option<&'a RenderTexture>) -> [Option<RenderPassColorAttachment>; 5]{
        [
            screen.unwrap_or(&self.rt_color).to_color_attachment(),
            self.rt_deflection.to_color_attachment(),
            self.rt_color_change.to_color_attachment(),
            self.rt_color_uncertainty.to_color_attachment(),
            self.rt_covariances.to_color_attachment(),
        ]
    }
}

pub enum Slot {
    Empty,
    Rgb {
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
    // XXX: Stereo
}
impl Default for Slot {
    fn default() -> Self {
        Slot::Empty
    }
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
        Self {
            input,
            output,
        }
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

    pub fn to_color_input(self, _window: &Window) -> Self {
        match self.input {
            Slot::Empty => {
                panic!("Input expected");
            }
            Slot::Rgb { .. } => self,
            Slot::RgbDepth {
                color_source, color_target, deflection_source, deflection_target, color_change_source, color_change_target, color_uncertainty_source, color_uncertainty_target, covariances_source, covariances_target, ..
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
                    covariances_target
                },
                output: self.output,
            },
        }
    }

    pub fn to_color_depth_input(self, _window: &Window) -> Self {
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

    pub fn to_color_output(self, window: &Window) -> Self {
        match self.output {
            Slot::Empty => {
                // Guess output size, based on input.
                let (width, height) = match &self.input {
                    Slot::Empty => {
                        panic!("Input expected");
                    }
                    Slot::Rgb { color_target, .. } => (color_target.width, color_target.height),
                    Slot::RgbDepth { color_target, .. } => (color_target.width, color_target.height),
                };
                let device = window.device().borrow_mut();
                let color_target = create_color_rt(&device, width, height, Some("to_color_output color"));
                let deflection_target = create_highp_rt(&device, width, height, Some("to_color_output deflection"));
                let color_change_target = create_highp_rt(&device, width, height, Some("to_color_output color_change"));
                let color_uncertainty_target = create_highp_rt(&device, width, height, Some("to_color_output color_uncertainty"));
                let covariances_target = create_highp_rt(&device, width, height, Some("to_color_output covariances"));

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
                        covariances_target
                    },
                }
            }
            Slot::Rgb { .. } => self,
            Slot::RgbDepth {
                color_source, color_target, deflection_source, deflection_target, color_change_source, color_change_target, color_uncertainty_source, color_uncertainty_target, covariances_source, covariances_target, ..
            } => {
                Self {
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
                    covariances_target
                },
            }},
        }
    }

    // pub fn to_color_depth_output(self, window: &Window) -> Self {
    //     match self.output {
    //         Slot::Empty => {
    //             // Guess output, based on input.
    //             let (width, height, ..) = match &self.input {
    //                 Slot::Empty => {
    //                     panic!("Input expected");
    //                 }
    //                 Slot::Rgb { color, .. } => color.get_dimensions(),
    //                 Slot::RgbDepth { color, .. } => color.get_dimensions(),
    //             };
    //             let mut factory = window.factory().borrow_mut();
    //             let (color, color_view) = create_texture_render_target::<ColorFormat>(
    //                 &mut factory,
    //                 width as u32,
    //                 height as u32,
    //             );
    //             let (depth, depth_view) = create_texture_render_target::<DepthFormat>(
    //                 &mut factory,
    //                 width as u32,
    //                 height as u32,
    //             );
    //             let (deflection, deflection_view) = create_texture_render_target::<Rgba32F>(
    //                 &mut factory,
    //                 width as u32,
    //                 height as u32,
    //             );
    //             let (color_change_source, color_change_target) = create_texture_render_target::<Rgba32F>(
    //                 &mut factory,
    //                 width as u32,
    //                 height as u32,
    //             );
    //             let (color_uncertainty_source, color_uncertainty_target) = create_texture_render_target::<Rgba32F>(
    //                 &mut factory,
    //                 width as u32,
    //                 height as u32,
    //             );
    //             let (covariances_source, covariances_target) = create_texture_render_target::<Rgba32F>(
    //                 &mut factory,
    //                 width as u32,
    //                 height as u32,
    //             );

    //             Self {
    //                 input: self.input,
    //                 output: Slot::RgbDepth {
    //                     color,
    //                     color_view: color_view,
    //                     depth,
    //                     depth_view,
    //                     deflection,
    //                     deflection_view,
    //                     color_change_source, 
    //                     color_change_target, 
    //                     color_uncertainty_source, 
    //                     color_uncertainty_target,                    
    //                     covariances_source,
    //                     covariances_target
    //                 },
    //             }
    //         }
    //         Slot::Rgb {                 
    //             color, color_view, deflection, deflection_view, color_change_source, color_change_target, color_uncertainty_source, color_uncertainty_target, covariances_source, covariances_target, ..
    //         } => {
    //             // Guess missing depth, based on color.
    //             let mut factory = window.factory().borrow_mut();
    //             let (width, height, ..) = color.get_dimensions();
    //             let (depth, depth_view) = create_texture_render_target::<DepthFormat>(
    //                 &mut factory,
    //                 width as u32,
    //                 height as u32,
    //             );
    //             Self {
    //                 input: self.input,
    //                 output: Slot::RgbDepth {
    //                     color,
    //                     color_view: color_view.expect("Shader resource expected"),
    //                     depth,
    //                     depth_view,
    //                     deflection,
    //                     deflection_view,
    //                     color_change_source, 
    //                     color_change_target, 
    //                     color_uncertainty_source, 
    //                     color_uncertainty_target,
    //                     covariances_source,
    //                     covariances_target
    //                 },
    //             }
    //         }
    //         Slot::RgbDepth { .. } => self,
    //     }
    // }

    pub fn emplace_color_output(self, window: &Window, width: u32, height: u32) -> Self {
        let device = window.device().borrow_mut();
        let color_target = create_color_rt(&device, width, height, Some("emplace_color_output color"));
        let deflection_target = create_highp_rt(&device, width, height, Some("emplace_color_output deflection"));
        let color_change_target = create_highp_rt(&device, width, height, Some("emplace_color_output color_change"));
        let color_uncertainty_target = create_highp_rt(&device, width, height, Some("emplace_color_output color_uncertainty"));
        let covariances_target = create_highp_rt(&device, width, height, Some("emplace_color_output covariances"));

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
                covariances_target
            },
        }
    }

    pub fn emplace_color_depth_output(self, window: &Window, width: u32, height: u32) -> Self {
        let device = window.device().borrow_mut();
        let color_target = create_color_rt(&device, width, height, Some("emplace_color_depth_output color"));
        let depth_target = create_depth_rt(&device, width, height, Some("emplace_color_depth_output depth"));
        let deflection_target = create_highp_rt(&device, width, height, Some("emplace_color_depth_output deflection"));
        let color_change_target = create_highp_rt(&device, width, height, Some("emplace_color_depth_output color_change"));
        let color_uncertainty_target = create_highp_rt(&device, width, height, Some("emplace_color_depth_output color_uncertainty"));
        let covariances_target = create_highp_rt(&device, width, height, Some("emplace_color_depth_output covariances"));

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
                covariances_target
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
                (
                    color_source.clone(),
                    bind_group,
                )
            }
        }
    }

    pub fn as_color_depth_source(&self, device: &wgpu::Device) -> ((Texture, BindGroup), (Texture, BindGroup)) {
        match &self.input {
            Slot::Empty | Slot::Rgb { .. } => {
                panic!("RGBD input expected");
            }
            Slot::RgbDepth { color_source, depth_source, .. } => {
                let (_, bind_group_color) = color_source.create_bind_group(device);
                let (_, bind_group_depth) = depth_source.create_bind_group(device);
                ((
                    color_source.clone(),
                    bind_group_color,
                ),
                (
                    depth_source.clone(),
                    bind_group_depth,
                ))
            }
        }
    }

    pub fn as_deflection_source(&self, device: &wgpu::Device) -> (Texture, BindGroup) {
        match &self.input {
            Slot::Empty | Slot::RgbDepth { .. } => {
                panic!("RGB input expected");
            }
            Slot::Rgb { deflection_source, .. } => {
                let (_, bind_group) = deflection_source.create_bind_group(device);
                (
                    deflection_source.clone(),
                    bind_group,
                )
            }
        }
    }

    pub fn as_color_change_source(&self, device: &wgpu::Device) -> (Texture, BindGroup) {
        match &self.input {
            Slot::Empty | Slot::RgbDepth { .. } => {
                panic!("RGB input expected");
            }
            Slot::Rgb { color_change_source, .. } => {
                let (_, bind_group) = color_change_source.create_bind_group(device);
                (
                    color_change_source.clone(),
                    bind_group,
                )
            }
        }
    }

    pub fn as_color_uncertainty_source(&self, device: &wgpu::Device) -> (Texture, BindGroup) {
        match &self.input {
            Slot::Empty | Slot::RgbDepth { .. } => {
                panic!("RGB input expected");
            }
            Slot::Rgb { color_uncertainty_source, .. } => {
                let (_, bind_group) = color_uncertainty_source.create_bind_group(device);
                (
                    color_uncertainty_source.clone(),
                    bind_group,
                )
            }
        }
    }

    pub fn as_covariances_source(&self, device: &wgpu::Device) -> (Texture, BindGroup) {
        match &self.input {
            Slot::Empty | Slot::RgbDepth { .. } => {
                panic!("RGB input expected");
            }
            Slot::Rgb { covariances_source, .. } => {
                let (_, bind_group) = covariances_source.create_bind_group(device);
                (
                    covariances_source.clone(),
                    bind_group,
                )
            }
        }
    }
    
    pub fn as_all_colors_source(&self, device: &wgpu::Device) -> BindGroup {
        match &self.input {
            Slot::Empty | Slot::RgbDepth { .. } => {
                panic!("RGB input expected");
            }
            Slot::Rgb { color_source, deflection_source, color_change_source, color_uncertainty_source, covariances_source, .. } => {
                create_textures_bind_group(
                    device,
                    &[
                        color_source,
                        deflection_source,
                        color_change_source,
                        color_uncertainty_source,
                        covariances_source,
                    ]).1
            }
        }
    }

    pub fn as_color_target(&self) -> RenderTexture{
        match &self.output {
            Slot::Empty | Slot::RgbDepth { .. } => {
                panic!("RGB output expected");
            }
            Slot::Rgb { color_target, .. } => color_target.clone(),
        }
    }
    
    pub fn as_color_depth_target(&self) -> (RenderTexture, RenderTexture){
        match &self.output {
            Slot::Empty | Slot::Rgb { .. } => {
                panic!("RGBD output expected");
            }
            Slot::RgbDepth { color_target, depth_target, .. } => (color_target.clone(), depth_target.clone()),
        }
    }

    pub fn as_deflection_target(&self) -> RenderTexture{
        match &self.output {
            Slot::Empty | Slot::RgbDepth { .. } => {
                panic!("RGB output expected");
            }
            Slot::Rgb { deflection_target, .. } => deflection_target.clone(),
        }
    }

    pub fn as_color_change_target(&self) -> RenderTexture{
        match &self.output {
            Slot::Empty | Slot::RgbDepth { .. } => {
                panic!("RGB output expected");
            }
            Slot::Rgb { color_change_target, .. } => color_change_target.clone(),
        }
    }

    pub fn as_color_uncertainty_target(&self) -> RenderTexture{
        match &self.output {
            Slot::Empty | Slot::RgbDepth { .. } => {
                panic!("RGB output expected");
            }
            Slot::Rgb { color_uncertainty_target, .. } => color_uncertainty_target.clone(),
        }
    }

    pub fn as_covariances_target(&self) -> RenderTexture{
        match &self.output {
            Slot::Empty | Slot::RgbDepth { .. } => {
                panic!("RGB output expected");
            }
            Slot::Rgb { covariances_target, .. } => covariances_target.clone(),
        }
    }

    pub fn as_all_target(
        &self,
    ) -> ColorDepthTargets {
        match &self.output {
            Slot::Empty | Slot::Rgb { .. } => {
                panic!("RGBD output expected");
            }
            Slot::RgbDepth { color_target, depth_target, deflection_target, color_change_target, color_uncertainty_target, covariances_target, .. } 
            => ColorDepthTargets{
                rt_color: color_target.clone(),
                rt_depth: depth_target.clone(),
                rt_deflection: deflection_target.clone(),
                rt_color_change: color_change_target.clone(),
                rt_color_uncertainty: color_uncertainty_target.clone(),
                rt_covariances: covariances_target.clone()
            },
        }
    }

    pub fn as_all_colors_target(
        &self,
    ) -> ColorTargets {
        match &self.output {
            Slot::Empty | Slot::RgbDepth { .. } => {
                panic!("RGB output expected");
            }
            Slot::Rgb { color_target, deflection_target, color_change_target, color_uncertainty_target, covariances_target, .. } 
            => ColorTargets{
                rt_color: color_target.clone(),
                rt_deflection: deflection_target.clone(),
                rt_color_change: color_change_target.clone(),
                rt_color_uncertainty: color_uncertainty_target.clone(),
                rt_covariances: covariances_target.clone()
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

pub struct WellKnownSlots{
    original_image: RefCell<Option<Texture>>
} 

impl WellKnownSlots{
    pub fn new() -> Self{
        WellKnownSlots{
            original_image: RefCell::new(None)
        }
    }

    pub fn get_original(
        &self,
    ) -> Option<Texture>
     {
        let guard =  RefCell::borrow(&self.original_image);
        match *guard{
            Some(ref original_image) => {
                Some(original_image.clone())
            },
            None => None
        }
    }
    pub fn set_original(
        &self,
        view: Texture
    ) {       
        let mut guard = RefCell::borrow_mut(&self.original_image);
        *guard =  Some(view.clone());
    }
}