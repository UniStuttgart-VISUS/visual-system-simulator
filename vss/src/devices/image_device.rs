use std::cell::RefCell;
use std::fs::File;

use super::*;
use crate::config::*;
use crate::pipeline::*;

/// A device for static RGBA image data.
pub struct ImageDevice {
    window: WindowDevice,
    input_rgba: RefCell<gfx::handle::Texture<gfx_device_gl::Resources, RgbSurfaceFormat>>,
    input_view: RefCell<DeviceSource>,
    output: String,
}

impl ImageDevice {
    pub fn new(config: &Config) -> Self {
        let window = WindowDevice::new(&config);
        let (input_rgba, rgba) =
            load_texture(&mut window.factory().borrow_mut(), load(&config.input)).unwrap();

        ImageDevice {
            window,
            input_rgba: RefCell::new(input_rgba),
            input_view: RefCell::new(DeviceSource::Rgb { rgba8: rgba }),
            output: config.output.clone(),
        }
    }

    pub fn upload_rgba(&mut self, rgba8: &[u8], width: usize, height: usize) {
        let factory = &mut self.factory().borrow_mut() as &mut gfx_device_gl::Factory;
        let encoder = &mut self.encoder().borrow_mut();

        // Test if texture size should change.
        let info = self.input_rgba.borrow().get_info().to_image_info(0);
        if width != info.width as usize || height != info.height as usize {
            let data = vec![0; width * height * 4].into_boxed_slice();
            let (input_rgba, rgba) =
                load_texture_from_bytes(factory, data, width as u32, height as u32).unwrap();
            self.input_rgba.replace(input_rgba);
            self.input_view.replace(DeviceSource::Rgb { rgba8: rgba });
        }

        // Update texture pixels.
        update_texture(
            encoder,
            &self.input_rgba.borrow(),
            [width as u16, height as u16],
            [0, 0],
            &*rgba8,
        );
    }
}

impl Device for ImageDevice {
    fn factory(&self) -> &RefCell<DeviceFactory> {
        self.window.factory()
    }

    fn encoder(&self) -> &RefCell<DeviceEncoder> {
        self.window.encoder()
    }

    fn gaze(&self) -> DeviceGaze {
        self.window.gaze()
    }

    fn source(&self) -> &RefCell<DeviceSource> {
        &self.input_view
    }

    fn target(&self) -> &RefCell<DeviceTarget> {
        self.window.target()
    }

    fn begin_frame(&self) {
        self.window.begin_frame();
    }

    fn end_frame(&self, done: &mut bool) {
        self.window.end_frame(done);

        if !self.output.is_empty() {
            let rgb_data = self.window.download_rgb();

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
