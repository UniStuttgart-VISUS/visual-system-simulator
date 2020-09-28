use std::cell::RefCell;

use gfx_device_gl::Resources;

use crate::window::*;
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
pub struct BufferToYuv {
    texture_y: RefCell<gfx::handle::Texture<Resources, YuvSurfaceFormat>>,
    texture_u: RefCell<gfx::handle::Texture<Resources, YuvSurfaceFormat>>,
    texture_v: RefCell<gfx::handle::Texture<Resources, YuvSurfaceFormat>>,
    view: RefCell<DeviceSource>,
}


    impl BufferToYuv {
    pub fn upload_yuv(&self, buffer: YUVBuffer) {
        let factory = &mut self.factory().borrow_mut();
        let encoder = &mut self.encoder().borrow_mut();

        // Test if texture size should change.
        let info = self.texture_y.borrow().get_info().to_image_info(0);
        if buffer.width != info.width as usize || buffer.height != info.height as usize {
            let (texture_y, y) = load_single_channel_texture_from_bytes(
                factory,
                buffer.pixels_y.clone(),
                buffer.width as u32,
                buffer.height as u32,
            )
            .unwrap();
            let (texture_u, u) = load_single_channel_texture_from_bytes(
                factory,
                buffer.pixels_u.clone(),
                (buffer.width / 2) as u32,
                (buffer.height / 2) as u32,
            )
            .unwrap();

            let (texture_v, v) = load_single_channel_texture_from_bytes(
                factory,
                buffer.pixels_v.clone(),
                (buffer.width / 2) as u32,
                (buffer.height / 2) as u32,
            )
            .unwrap();

            self.texture_y.replace(texture_y);
            self.texture_u.replace(texture_u);
            self.texture_v.replace(texture_v);
            self.view.replace(DeviceSource::Yuv { y, u, v });
        }

        // Update texture pixels.
        let size = [buffer.width as u16, buffer.height as u16];
        let half_size = [(buffer.width / 2) as u16, (buffer.height / 2) as u16];
        let offset = [0, 0];
        update_single_channel_texture(
            encoder,
            &self.texture_y.borrow(),
            size,
            offset,
            &buffer.pixels_y,
        );
        update_single_channel_texture(
            encoder,
            &self.texture_u.borrow(),
            half_size,
            offset,
            &buffer.pixels_u,
        );
        update_single_channel_texture(
            encoder,
            &self.texture_v.borrow(),
            half_size,
            offset,
            &buffer.pixels_v,
        );
    }

    pub fn download_rgb(&self) -> RGBBuffer {
        self.pipeline().borrow().download_rgb(&self.window)
    }
}


impl Node for BufferToYuv {
    fn build(factory: &mut gfx_device_gl::Factory) -> Self {
        let dummy_width = 1;
        let dummy_height = 1;
        let dummy_pixels = vec![128; dummy_width * dummy_height].into_boxed_slice();

        let (texture_y, y) = load_single_channel_texture_from_bytes(
            factory,
            dummy_pixels.clone(),
            dummy_width as u32,
            dummy_height as u32,
        )
        .unwrap();
        let (texture_u, u) = load_single_channel_texture_from_bytes(
            factory,
            dummy_pixels.clone(),
            dummy_width as u32,
            dummy_height as u32,
        )
        .unwrap();
        let (texture_v, v) = load_single_channel_texture_from_bytes(
            factory,
            dummy_pixels,
            dummy_width as u32,
            dummy_height as u32,
        )
        .unwrap();

        BufferToYuv { 
            texture_y: RefCell::new(texture_y),
            texture_u: RefCell::new(texture_u),
            texture_v: RefCell::new(texture_v),
            view: RefCell::new(DeviceSource::Yuv { y, u, v }),
        }
    }

    fn update_io<'a>(
        &'a mut self,
        factory: &mut gfx_device_gl::Factory,
                source: &DeviceSource,
        target_candidate: &'a DeviceTarget,
    ) ->  &'a DeviceTarget {
        &self.view
    }
}

 