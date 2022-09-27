use super::*;
use std::cell::RefCell;

pub static ColorFormat:wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;
pub static HighpFormat:wgpu::TextureFormat = wgpu::TextureFormat::Rgba32Float;
pub static DepthFormat:wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;

pub enum Slot {
    Empty,
    Rgb {
        color: Texture,
        // color: gfx::handle::RenderTargetView<gfx_device_gl::Resources, ColorFormat>,
        // color_view: Option<gfx::handle::ShaderResourceView<gfx_device_gl::Resources, [f32; 4]>>, //TODO: drop last component?
        // deflection: gfx::handle::RenderTargetView<gfx_device_gl::Resources, Rgba32F>,
        // deflection_view: gfx::handle::ShaderResourceView<gfx_device_gl::Resources, [f32; 4]>,
        // color_change: gfx::handle::RenderTargetView<gfx_device_gl::Resources, Rgba32F>,
        // color_change_view: gfx::handle::ShaderResourceView<gfx_device_gl::Resources, [f32; 4]>,
        // color_uncertainty: gfx::handle::RenderTargetView<gfx_device_gl::Resources, Rgba32F>,
        // color_uncertainty_view: gfx::handle::ShaderResourceView<gfx_device_gl::Resources, [f32; 4]>,
        // covariances: gfx::handle::RenderTargetView<gfx_device_gl::Resources, Rgba32F>,
        // covariances_view: gfx::handle::ShaderResourceView<gfx_device_gl::Resources, [f32; 4]>,
    },
    RgbDepth {
        // color: gfx::handle::RenderTargetView<gfx_device_gl::Resources, ColorFormat>,
        // color_view: gfx::handle::ShaderResourceView<gfx_device_gl::Resources, [f32; 4]>, //TODO: drop last component?
        // depth: gfx::handle::RenderTargetView<gfx_device_gl::Resources, DepthFormat>,
        // depth_view: gfx::handle::ShaderResourceView<gfx_device_gl::Resources, f32>,
        // deflection: gfx::handle::RenderTargetView<gfx_device_gl::Resources, Rgba32F>,
        // deflection_view: gfx::handle::ShaderResourceView<gfx_device_gl::Resources, [f32; 4]>,
        // color_change: gfx::handle::RenderTargetView<gfx_device_gl::Resources, Rgba32F>,
        // color_change_view: gfx::handle::ShaderResourceView<gfx_device_gl::Resources, [f32; 4]>,
        // color_uncertainty: gfx::handle::RenderTargetView<gfx_device_gl::Resources, Rgba32F>,
        // color_uncertainty_view: gfx::handle::ShaderResourceView<gfx_device_gl::Resources, [f32; 4]>,
        // covariances: gfx::handle::RenderTargetView<gfx_device_gl::Resources, Rgba32F>,
        // covariances_view: gfx::handle::ShaderResourceView<gfx_device_gl::Resources, [f32; 4]>,
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
    // sampler: gfx::handle::Sampler<gfx_device_gl::Resources>,
}

impl NodeSlots {
    pub fn new(window: &Window) -> Self {
        Self {
            input: Slot::default(),
            output: Slot::default(),
            // sampler: window.factory().borrow_mut().create_sampler_linear(),
        }
    }

    pub fn new_io(window: &Window, input: Slot, output: Slot) -> Self {
        Self {
            input,
            output,
            // sampler: window.factory().borrow_mut().create_sampler_linear(),
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
            // sampler: self.sampler,
        }
    }

    // pub fn to_color_input(self, _window: &Window) -> Self {
    //     match self.input {
    //         Slot::Empty => {
    //             panic!("Input expected");
    //         }
    //         Slot::Rgb { .. } => self,
    //         Slot::RgbDepth {
    //             color, color_view, deflection, deflection_view, color_change, color_change_view, color_uncertainty, color_uncertainty_view, covariances, covariances_view, ..
    //         } => Self {
    //             input: Slot::Rgb {
    //                 color,
    //                 color_view: Some(color_view),
    //                 deflection: deflection,
    //                 deflection_view,
    //                 color_change, 
    //                 color_change_view, 
    //                 color_uncertainty, 
    //                 color_uncertainty_view,
    //                 covariances,
    //                 covariances_view
    //             },
    //             output: self.output,
    //             sampler: self.sampler,
    //         },
    //     }
    // }

    // pub fn to_color_depth_input(self, _window: &Window) -> Self {
    //     match self.input {
    //         Slot::Empty => {
    //             panic!("Input expected");
    //         }
    //         Slot::Rgb { .. } => {
    //             panic!("RGB input cannot be extended with depth");
    //         }
    //         Slot::RgbDepth { .. } => self,
    //     }
    // }

    // pub fn to_color_output(self, window: &Window) -> Self {
    //     match self.output {
    //         Slot::Empty => {
    //             // Guess output size, based on input.
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
    //             let (deflection, deflection_view) = create_texture_render_target::<Rgba32F>(
    //                 &mut factory,
    //                 width as u32,
    //                 height as u32,
    //             );
    //             let (color_change, color_change_view) = create_texture_render_target::<Rgba32F>(
    //                 &mut factory,
    //                 width as u32,
    //                 height as u32,
    //             );
    //             let (color_uncertainty, color_uncertainty_view) = create_texture_render_target::<Rgba32F>(
    //                 &mut factory,
    //                 width as u32,
    //                 height as u32,
    //             );
    //             let (covariances, covariances_view) = create_texture_render_target::<Rgba32F>(
    //                 &mut factory,
    //                 width as u32,
    //                 height as u32,
    //             );

    //             Self {
    //                 input: self.input,
    //                 output: Slot::Rgb {
    //                     color,
    //                     color_view: Some(color_view),
    //                     deflection,
    //                     deflection_view,
    //                     color_change, 
    //                     color_change_view, 
    //                     color_uncertainty, 
    //                     color_uncertainty_view,
    //                     covariances,
    //                     covariances_view
    //                 },
    //                 sampler: self.sampler,
    //             }
    //         }
    //         Slot::Rgb { .. } => self,
    //         Slot::RgbDepth {
    //             color, color_view, deflection, deflection_view, color_change, color_change_view, color_uncertainty, color_uncertainty_view, covariances, covariances_view, ..
    //         } => {
    //             Self {
    //             input: self.input,
    //             output: Slot::Rgb {
    //                 color,
    //                 color_view: Some(color_view),                        
    //                 deflection,
    //                 deflection_view,
    //                 color_change, 
    //                 color_change_view, 
    //                 color_uncertainty, 
    //                 color_uncertainty_view,
    //                 covariances,
    //                 covariances_view
    //             },
    //             sampler: self.sampler,
    //         }},
    //     }
    // }

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
    //             let (color_change, color_change_view) = create_texture_render_target::<Rgba32F>(
    //                 &mut factory,
    //                 width as u32,
    //                 height as u32,
    //             );
    //             let (color_uncertainty, color_uncertainty_view) = create_texture_render_target::<Rgba32F>(
    //                 &mut factory,
    //                 width as u32,
    //                 height as u32,
    //             );
    //             let (covariances, covariances_view) = create_texture_render_target::<Rgba32F>(
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
    //                     color_change, 
    //                     color_change_view, 
    //                     color_uncertainty, 
    //                     color_uncertainty_view,                    
    //                     covariances,
    //                     covariances_view
    //                 },
    //                 sampler: self.sampler,
    //             }
    //         }
    //         Slot::Rgb {                 
    //             color, color_view, deflection, deflection_view, color_change, color_change_view, color_uncertainty, color_uncertainty_view, covariances, covariances_view, ..
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
    //                     color_change, 
    //                     color_change_view, 
    //                     color_uncertainty, 
    //                     color_uncertainty_view,
    //                     covariances,
    //                     covariances_view
    //                 },
    //                 sampler: self.sampler,
    //             }
    //         }
    //         Slot::RgbDepth { .. } => self,
    //     }
    // }

    // pub fn emplace_color_output(self, window: &Window, width: u32, height: u32) -> Self {
    //     let mut factory = window.factory().borrow_mut();
    //     let (color, color_view) = create_texture_render_target::<ColorFormat>(
    //         &mut factory,
    //         width,
    //         height,
    //     );
    //     let (deflection, deflection_view) = create_texture_render_target::<Rgba32F>(
    //         &mut factory,
    //         width as u32,
    //         height as u32,
    //     );
    //     let (color_change, color_change_view) = create_texture_render_target::<Rgba32F>(
    //         &mut factory,
    //         width as u32,
    //         height as u32,
    //     );
    //     let (color_uncertainty, color_uncertainty_view) = create_texture_render_target::<Rgba32F>(
    //         &mut factory,
    //         width as u32,
    //         height as u32,
    //     );
    //     let (covariances, covariances_view) = create_texture_render_target::<Rgba32F>(
    //         &mut factory,
    //         width as u32,
    //         height as u32,
    //     );

    //     Self {
    //         input: self.input,
    //         output: Slot::Rgb {
    //             color,
    //             color_view: Some(color_view),
    //             deflection,
    //             deflection_view,
    //             color_change, 
    //             color_change_view, 
    //             color_uncertainty, 
    //             color_uncertainty_view,
    //             covariances,
    //             covariances_view
    //         },
    //         sampler: self.sampler,
    //     }
    // }

    // pub fn emplace_color_depth_output(self, window: &Window, width: u32, height: u32) -> Self {
    //     let mut factory = window.factory().borrow_mut();

    //     let (color, color_view) = create_texture_render_target::<ColorFormat>(
    //         &mut factory,
    //         width,
    //         height,
    //     );
    //     let (depth, depth_view) = create_texture_render_target::<DepthFormat>(
    //         &mut factory,
    //         width,
    //         height,
    //     );
    //     let (deflection, deflection_view) = create_texture_render_target::<Rgba32F>(
    //         &mut factory,
    //         width as u32,
    //         height as u32,
    //     );
    //     let (color_change, color_change_view) = create_texture_render_target::<Rgba32F>(
    //         &mut factory,
    //         width as u32,
    //         height as u32,
    //     );
    //     let (color_uncertainty, color_uncertainty_view) = create_texture_render_target::<Rgba32F>(
    //         &mut factory,
    //         width as u32,
    //         height as u32,
    //     );
    //     let (covariances, covariances_view) = create_texture_render_target::<Rgba32F>(
    //         &mut factory,
    //         width as u32,
    //         height as u32,
    //     );

    //     Self {
    //         input: self.input,
    //         output: Slot::RgbDepth {
    //             color,
    //             color_view,
    //             depth,
    //             depth_view,
    //             deflection,
    //             deflection_view,
    //             color_change, 
    //             color_change_view, 
    //             color_uncertainty, 
    //             color_uncertainty_view,
    //             covariances,
    //             covariances_view
    //         },
    //         sampler: self.sampler,
    //     }
    // }

    // pub fn as_deflection(&self) -> gfx::handle::RenderTargetView<gfx_device_gl::Resources, Rgba32F> {
    //     match &self.output {
    //         Slot::Empty => {
    //             panic!("RGB output expected");
    //         }
    //         Slot::RgbDepth { deflection, .. } => deflection.clone(),
    //         Slot::Rgb { deflection, .. } => deflection.clone(),
    //     }
    // }

    // pub fn as_color_change(&self) -> gfx::handle::RenderTargetView<gfx_device_gl::Resources, Rgba32F> {
    //     match &self.output {
    //         Slot::Empty => {
    //             panic!("RGB output expected");
    //         }
    //         Slot::RgbDepth { color_change, .. } => color_change.clone(),
    //         Slot::Rgb { color_change, .. } => color_change.clone(),
    //     }
    // }

    // pub fn as_color_uncertainty(&self) -> gfx::handle::RenderTargetView<gfx_device_gl::Resources, Rgba32F> {
    //     match &self.output {
    //         Slot::Empty => {
    //             panic!("RGB output expected");
    //         }
    //         Slot::RgbDepth { color_uncertainty, .. } => color_uncertainty.clone(),
    //         Slot::Rgb { color_uncertainty, .. } => color_uncertainty.clone(),
    //     }
    // }

    // pub fn as_covariances(&self) -> gfx::handle::RenderTargetView<gfx_device_gl::Resources, Rgba32F> {
    //     match &self.output {
    //         Slot::Empty => {
    //             panic!("RGB output expected");
    //         }
    //         Slot::RgbDepth { covariances, .. } => covariances.clone(),
    //         Slot::Rgb { covariances, .. } => covariances.clone(),
    //     }
    // }

    // pub fn as_color(&self) -> gfx::handle::RenderTargetView<gfx_device_gl::Resources, ColorFormat> {
    //     match &self.output {
    //         Slot::Empty | Slot::RgbDepth { .. } => {
    //             panic!("RGB output expected");
    //         }
    //         Slot::Rgb { color, .. } => color.clone(),
    //     }
    // }

    // pub fn as_color_view(
    //     &self,
    // ) -> (
    //     gfx::handle::ShaderResourceView<gfx_device_gl::Resources, [f32; 4]>,
    //     gfx::handle::Sampler<gfx_device_gl::Resources>,
    // ) {
    //     match &self.input {
    //         Slot::Empty | Slot::RgbDepth { .. } => {
    //             panic!("RGB input expected");
    //         }
    //         Slot::Rgb { color_view, .. } => (
    //             color_view.clone().expect("Shader resource expected"),
    //             self.sampler.clone(),
    //         ),
    //     }
    // }

    // pub fn as_deflection_view(
    //     &self,
    // ) -> (
    //     gfx::handle::ShaderResourceView<gfx_device_gl::Resources, [f32; 4]>,
    //     gfx::handle::Sampler<gfx_device_gl::Resources>,
    // ) {
    //     match &self.input {
    //         Slot::Empty => {
    //             panic!("RGB input expected");
    //         }
    //         Slot::Rgb { deflection_view, .. } => (
    //             deflection_view.clone(),
    //             self.sampler.clone(),
    //         ),
    //         Slot::RgbDepth { deflection_view,.. } => (
    //             deflection_view.clone(),
    //             self.sampler.clone(),
    //         ),
    //     }
    // }

    // pub fn as_color_change_view(
    //     &self,
    // ) -> (
    //     gfx::handle::ShaderResourceView<gfx_device_gl::Resources, [f32; 4]>,
    //     gfx::handle::Sampler<gfx_device_gl::Resources>,
    // ) {
    //     match &self.input {
    //         Slot::Empty => {
    //             panic!("RGB input expected");
    //         }
    //         Slot::Rgb { color_change_view, .. } => (
    //             color_change_view.clone(),
    //             self.sampler.clone(),
    //         ),
    //         Slot::RgbDepth { color_change_view,.. } => (
    //             color_change_view.clone(),
    //             self.sampler.clone(),
    //         ),
    //     }
    // }
    // pub fn as_color_uncertainty_view(
    //     &self,
    // ) -> (
    //     gfx::handle::ShaderResourceView<gfx_device_gl::Resources, [f32; 4]>,
    //     gfx::handle::Sampler<gfx_device_gl::Resources>,
    // ) {
    //     match &self.input {
    //         Slot::Empty => {
    //             panic!("RGB input expected");
    //         }
    //         Slot::Rgb { color_uncertainty_view, .. } => (
    //             color_uncertainty_view.clone(),
    //             self.sampler.clone(),
    //         ),
    //         Slot::RgbDepth { color_uncertainty_view,.. } => (
    //             color_uncertainty_view.clone(),
    //             self.sampler.clone(),
    //         ),
    //     }
    // }

    // pub fn as_covariances_view(
    //     &self,
    // ) -> (
    //     gfx::handle::ShaderResourceView<gfx_device_gl::Resources, [f32; 4]>,
    //     gfx::handle::Sampler<gfx_device_gl::Resources>,
    // ) {
    //     match &self.input {
    //         Slot::Empty => {
    //             panic!("RGB input expected");
    //         }
    //         Slot::Rgb { covariances_view, .. } => (
    //             covariances_view.clone(),
    //             self.sampler.clone(),
    //         ),
    //         Slot::RgbDepth { covariances_view,.. } => (
    //             covariances_view.clone(),
    //             self.sampler.clone(),
    //         ),
    //     }
    // }

    // pub fn as_color_depth(
    //     &self,
    // ) -> (
    //     gfx::handle::RenderTargetView<gfx_device_gl::Resources, ColorFormat>,
    //     gfx::handle::RenderTargetView<gfx_device_gl::Resources, DepthFormat>,
    // ) {
    //     match &self.output {
    //         Slot::Empty | Slot::Rgb { .. } => {
    //             panic!("RGBD output expected");
    //         }
    //         Slot::RgbDepth { color, depth, .. } => (color.clone(), depth.clone()),
    //     }
    // }

    // pub fn as_all_output(
    //     &self,
    // ) -> (
    //     gfx::handle::RenderTargetView<gfx_device_gl::Resources, ColorFormat>,
    //     gfx::handle::RenderTargetView<gfx_device_gl::Resources, DepthFormat>,
    //     gfx::handle::RenderTargetView<gfx_device_gl::Resources, Rgba32F>,
    //     gfx::handle::RenderTargetView<gfx_device_gl::Resources, Rgba32F>,
    //     gfx::handle::RenderTargetView<gfx_device_gl::Resources, Rgba32F>,
    //     gfx::handle::RenderTargetView<gfx_device_gl::Resources, Rgba32F>,

    // ) {
    //     match &self.output {
    //         Slot::Empty | Slot::Rgb { .. } => {
    //             panic!("RGBD output expected");
    //         }
    //         Slot::RgbDepth { color, depth, deflection, color_change, color_uncertainty, covariances, .. } 
    //         => (color.clone(), depth.clone(), deflection.clone(), color_change.clone(), color_uncertainty.clone(),covariances.clone()),
    //     }
    // }

//     pub fn as_color_depth_view(
//         &self,
//     ) -> (
//         (
//             gfx::handle::ShaderResourceView<gfx_device_gl::Resources, [f32; 4]>,
//             gfx::handle::Sampler<gfx_device_gl::Resources>,
//         ),
//         (
//             gfx::handle::ShaderResourceView<gfx_device_gl::Resources, f32>,
//             gfx::handle::Sampler<gfx_device_gl::Resources>,
//         ),
//     ) {
//         match &self.input {
//             Slot::Empty | Slot::Rgb { .. } => {
//                 panic!("RGBD input expected");
//             }
//             Slot::RgbDepth {
//                 color_view,
//                 depth_view,
//                 ..
//             } => (
//                 (color_view.clone(), self.sampler.clone()),
//                 (depth_view.clone(), self.sampler.clone()),
//             ),
//         }
//     }

//     fn output_size(&self) -> [u32; 2] {
//         let dimensions = match &self.output {
//             Slot::Empty => {
//                 panic!("Output expected");
//             }
//             Slot::Rgb { color, .. } => color.get_dimensions(),
//             Slot::RgbDepth { color, .. } => color.get_dimensions(),
//         };

//         [dimensions.0 as u32, dimensions.1 as u32]
//     }

//     pub fn output_size_f32(&self) -> [f32; 2] {
//         let size = self.output_size();
//         [size[0] as f32, size[1] as f32]
//     }

//     pub fn input_size_f32(&self) -> [f32; 2] {
//         let dimensions = match &self.input {
//             Slot::Empty => {
//                 panic!("Output expected");
//             }
//             Slot::Rgb { color, .. } => color.get_dimensions(),
//             Slot::RgbDepth { color, .. } => color.get_dimensions(),
//         };

//         [dimensions.0 as f32, dimensions.1 as f32]
//     }

    
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

//     pub fn get_original(
//         &self,
//     ) -> Option<(gfx::handle::ShaderResourceView<gfx_device_gl::Resources, [f32; 4]>, gfx::handle::Sampler<gfx_device_gl::Resources>)>
//      {
//         let guard =  RefCell::borrow(&self.original_image);
//         match *guard{
//             Some(ref original_image) => {
//                 Some(original_image.clone())
//             },
//             None => None
//         }
//     }
//     pub fn set_original(
//         &self,
//         view: (gfx::handle::ShaderResourceView<gfx_device_gl::Resources, [f32; 4]>, gfx::handle::Sampler<gfx_device_gl::Resources>)
//     ) {       
//         let mut guard = RefCell::borrow_mut(&self.original_image);
//         *guard =  Some(view.clone());
//     }
}