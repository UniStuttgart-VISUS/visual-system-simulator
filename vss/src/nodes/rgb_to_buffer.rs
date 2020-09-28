use gfx;
use gfx::traits::FactoryExt;
use gfx::Factory;
use gfx_device_gl::CommandBuffer;
use gfx_device_gl::Resources;
use std::fs::File;
 
use crate::pipeline::*;

/// A device for static RGBA image data.
pub struct RgbToBuffer {
    output: Option<String>,
}

 
impl Node for RgbToBuffer {
    fn build(factory: &mut gfx_device_gl::Factory) -> Self {
 
        RgbToBuffer {
             output:None
        }
    }
    fn update_io(
        &mut self,
        factory: &mut gfx_device_gl::Factory,
        source: Option<DeviceSource>,
        target_candidate: (Option<DeviceSource>, Option<DeviceTarget>),
    ) -> (Option<DeviceSource>, Option<DeviceTarget>) {
        //target_candidate
    }

  
    fn update_values(
        &mut self,
        factory: &mut gfx_device_gl::Factory,
        values: &ValueMap,
        gaze: &DeviceGaze,
    ) {
     
       //self.output = values."image_out": config.output.clone()

    }

    fn render(&mut self, encoder: &mut gfx::Encoder<Resources, CommandBuffer>) {
        

        if self.output.is_some() {
            let rgb_data = self.pipeline().borrow().download_rgb(&self.window);

            let mut image_data: Vec<u8> = Vec::new();
            let encoder = image::png::PngEncoder::new(&mut image_data);
            let _res = encoder.encode(
                &rgb_data.pixels_rgb,
                rgb_data.width as u32,
                rgb_data.height as u32,
                image::ColorType::Rgb8,
            );
            use std::io::Write;
            let mut file = File::create(&self.output).expect("Unable to create file");
            file.write_all(&image_data).unwrap();
            println!("[image] writing to {}", self.output);
             
        }
 
    }

}

 