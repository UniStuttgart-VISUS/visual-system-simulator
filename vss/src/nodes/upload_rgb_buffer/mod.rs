use std::io::Cursor;
use std::path::Path;

use crate::pipeline::*;
use crate::window::Window;

/// A device for static RGBA image data.
pub struct UploadRgbBuffer {
    buffer_next: Option<RgbBuffer>,
    texture: Option<gfx::handle::Texture<gfx_device_gl::Resources, RgbSurfaceFormat>>,
    view: Option<DeviceSource>,
}

impl UploadRgbBuffer {
    pub fn has_image_extension<P>(path: P) -> bool
    where
        P: AsRef<Path>,
    {
        image::ImageFormat::from_path(path).is_ok()
    }

    pub fn enqueue_image(&mut self, cursor: Cursor<Vec<u8>>) {
        let reader = image::io::Reader::new(cursor)
            .with_guessed_format()
            .expect("Cursor io never fails");
        let img = reader.decode().unwrap().flipv().to_rgba();
        let (width, height) = img.dimensions();

        self.enqueue_buffer(RgbBuffer {
            width: width,
            height: height,
            pixels_rgb: img.into_raw().into_boxed_slice(),
        });
    }

    pub fn enqueue_buffer(&mut self, buffer: RgbBuffer) {
        // Test if we have to invalidate the texture.
        if let Some(texture) = &self.texture {
            let info = texture.get_info().to_image_info(0);
            if buffer.width != info.width as u32 || buffer.height != info.height as u32 {
                self.texture = None;
                self.view = None;
            }
        }

        self.buffer_next = Some(buffer);
    }
}

impl Node for UploadRgbBuffer {
    fn new(_window: &Window) -> Self {
        UploadRgbBuffer {
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
            let (texture, view) = load_texture_from_bytes(
                &mut factory,
                buffer.pixels_rgb.clone(),
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
