use std::cell::RefCell;

use gfx_device_gl::Resources;

use super::*;
use crate::config::*;
use crate::pipeline::*;

// A buffer representing color information.
//
// For YUV, the U anv C channels only have half width and height by convetion.
pub struct YUVBuffer {
    pub pixels_y: Box<[u8]>,
    pub pixels_u: Box<[u8]>,
    pub pixels_v: Box<[u8]>,
    pub width: usize,
    pub height: usize,
}

// A device for dynamic YUV video data.
pub struct YuvDevice {
    window: WindowDevice,
    input_y: RefCell<gfx::handle::Texture<Resources, YuvSurfaceFormat>>,
    input_u: RefCell<gfx::handle::Texture<Resources, YuvSurfaceFormat>>,
    input_v: RefCell<gfx::handle::Texture<Resources, YuvSurfaceFormat>>,
    input_view: RefCell<DeviceSource>,
}

impl YuvDevice {
    pub fn new(config: &Config) -> Self {
        let dummy_width = 1;
        let dummy_height = 1;
        let dummy_pixels = vec![128; dummy_width * dummy_height].into_boxed_slice();

        let window = WindowDevice::new(&config, true);
        let (input_y, y) = load_single_channel_texture_from_bytes(
            &mut window.factory().borrow_mut(),
            dummy_pixels.clone(),
            dummy_width as u32,
            dummy_height as u32,
        )
        .unwrap();
        let (input_u, u) = load_single_channel_texture_from_bytes(
            &mut window.factory().borrow_mut(),
            dummy_pixels.clone(),
            dummy_width as u32,
            dummy_height as u32,
        )
        .unwrap();
        let (input_v, v) = load_single_channel_texture_from_bytes(
            &mut window.factory().borrow_mut(),
            dummy_pixels,
            dummy_width as u32,
            dummy_height as u32,
        )
        .unwrap();

        YuvDevice {
            window,
            input_y: RefCell::new(input_y),
            input_u: RefCell::new(input_u),
            input_v: RefCell::new(input_v),
            input_view: RefCell::new(DeviceSource::Yuv { y, u, v }),
        }
    }

    pub fn upload_yuv(&self, buffer: YUVBuffer) {
        let factory = &mut self.factory().borrow_mut();
        let encoder = &mut self.encoder().borrow_mut();

        // Test if texture size should change.
        let info = self.input_y.borrow().get_info().to_image_info(0);
        if buffer.width != info.width as usize || buffer.height != info.height as usize {
            let (input_y, y) = load_single_channel_texture_from_bytes(
                factory,
                buffer.pixels_y.clone(),
                buffer.width as u32,
                buffer.height as u32,
            )
            .unwrap();
            let (input_u, u) = load_single_channel_texture_from_bytes(
                factory,
                buffer.pixels_u.clone(),
                (buffer.width / 2) as u32,
                (buffer.height / 2) as u32,
            )
            .unwrap();

            let (input_v, v) = load_single_channel_texture_from_bytes(
                factory,
                buffer.pixels_v.clone(),
                (buffer.width / 2) as u32,
                (buffer.height / 2) as u32,
            )
            .unwrap();

            self.input_y.replace(input_y);
            self.input_u.replace(input_u);
            self.input_v.replace(input_v);
            self.input_view.replace(DeviceSource::Yuv { y, u, v });
        }

        // Update texture pixels.
        let size = [buffer.width as u16, buffer.height as u16];
        let half_size = [(buffer.width / 2) as u16, (buffer.height / 2) as u16];
        let offset = [0, 0];
        update_single_channel_texture(
            encoder,
            &self.input_y.borrow(),
            size,
            offset,
            &buffer.pixels_y,
        );
        update_single_channel_texture(
            encoder,
            &self.input_u.borrow(),
            half_size,
            offset,
            &buffer.pixels_u,
        );
        update_single_channel_texture(
            encoder,
            &self.input_v.borrow(),
            half_size,
            offset,
            &buffer.pixels_v,
        );
    }

    pub fn download_rgb(&self) -> RGBBuffer {
        self.window.download_rgb()
    }
}

impl Device for YuvDevice {
    fn pipeline(&self) -> &RefCell<Pipeline> {
        self.window.pipeline()
    }

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

    fn render(&self) -> bool {
        self.window.render()
    }
}
