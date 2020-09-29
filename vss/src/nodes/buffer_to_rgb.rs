use std::io::Cursor;

use crate::pipeline::*;
use crate::window::Window;

/// A device for static RGBA image data.
pub struct BufferToRgb {
    buffer_next: Option<RGBBuffer>,
    texture: Option<gfx::handle::Texture<gfx_device_gl::Resources, RgbSurfaceFormat>>,
    view: Option<DeviceSource>,
}

impl BufferToRgb {
    pub fn enqueue_buffer(&mut self, data: Cursor<Vec<u8>>) {
        let img = image::load(data, image::ImageFormat::Png)
            .unwrap()
            .flipv()
            .to_rgba();
        let (width, height) = img.dimensions();

        // Test if we have to invalidate the texture.
        if let Some(texture) = &self.texture {
            let info = texture.get_info().to_image_info(0);
            if width != info.width as u32 || height != info.height as u32 {
                self.texture = None;
                self.view = None;
            }
        }

        self.buffer_next = Some(RGBBuffer {
            width: width as usize,
            height: height as usize,
            pixels_rgb: img.into_raw().into_boxed_slice(),
        });
    }
}

impl Node for BufferToRgb {
    fn new(_window: &Window) -> Self {
        BufferToRgb {
            buffer_next: None,
            texture: None,
            view: None,
        }
    }

    fn update_io(
        &mut self,
        window: &Window,
        _source: (Option<DeviceSource>, Option<DeviceTarget>),
        _target_candidate: (Option<DeviceSource>, Option<DeviceTarget>),
    ) -> (Option<DeviceSource>, Option<DeviceTarget>) {
        let mut factory = window.factory().borrow_mut();
        if let Some(buffer) = &self.buffer_next {
            let data = vec![0; buffer.width * buffer.height * 4].into_boxed_slice();
            let (texture, view) = load_texture_from_bytes(
                &mut factory,
                data,
                buffer.width as u32,
                buffer.height as u32,
            )
            .unwrap();
            self.texture = Some(texture);
            self.view = Some(DeviceSource::Rgb {
                width: buffer.width as u32,
                height: buffer.height as u32,
                rgba8: view,
            });
        }
        debug_assert!(self.view.is_some(), "A buffer must be set at least once");
        (self.view.clone(), None)
    }

    fn render(&mut self, window: &Window) {
        let mut encoder = window.encoder().borrow_mut();

        if let Some(texture) = &self.texture {
            if let Some(buffer) = self.buffer_next.take() {
                update_texture(
                    &mut encoder,
                    &texture,
                    [buffer.width as u16, buffer.height as u16],
                    [0, 0],
                    &*buffer.pixels_rgb,
                );
            }
        }
    }
}
