use super::*;
use std::path::Path;

pub type RgbBufferCb = Box<dyn FnOnce(RgbBuffer) + Send>;

enum Message {
    Buffer(RgbBuffer),
    Callback(Option<RgbBufferCb>),
}
/// A node that downloads RGB buffers.
pub struct DownloadRgbBuffer {
    input: Slot,
    tx: std::sync::mpsc::Sender<Message>,
}

impl DownloadRgbBuffer {
    pub fn set_buffer_cb(&mut self, cb: Option<RgbBufferCb>) {
        self.tx.send(Message::Callback(cb)).unwrap();
    }

    pub fn set_image_path<P>(&mut self, path: P, processed: std::sync::Arc<std::sync::RwLock<bool>>)
    where
        P: 'static + std::fmt::Debug + Send + AsRef<Path>,
    {
        let cb = Box::new(move |rgb_buffer: RgbBuffer| {
            let dir = path.as_ref().parent().unwrap();
            std::fs::create_dir_all(dir).expect("Unable to create directory");
            image::save_buffer(
                &path,
                &rgb_buffer.pixels_rgb,
                rgb_buffer.width as u32,
                rgb_buffer.height as u32,
                image::ColorType::Rgb8,
            )
            .expect("Unable to create file");
            {
                let mut processed = processed.write().unwrap();
                *processed = true;
            }
            println!("[image] written to {:?}", path);
        });
        self.set_buffer_cb(Some(cb));
    }
}

impl Node for DownloadRgbBuffer {
    fn new(_window: &Window) -> Self {
        let (tx, rx) = std::sync::mpsc::channel::<Message>();
        std::thread::spawn(move || {
            let mut callback: Option<RgbBufferCb> = None;
            while let Ok(message) = rx.recv() {
                match message {
                    Message::Buffer(rgb_buffer) => {
                        if let Some(cb) = callback.take() {
                            (cb)(rgb_buffer);
                        }
                    }
                    Message::Callback(new_callback) => {
                        callback = new_callback;
                    }
                }
            }
        });

        DownloadRgbBuffer {
            input: Slot::Empty,
            tx,
        }
    }

    fn negociate_slots(&mut self, _window: &Window, slots: NodeSlots) -> NodeSlots {
        self.input = slots.clone().take_input();
        slots.to_passthrough()
    }

    fn render(&mut self, window: &Window) {
        match &self.input {
            Slot::Rgb { color, .. } | Slot::RgbDepth { color, .. } => {
                use gfx::format::Formatted;
                use gfx::memory::Typed;

                let factory = &mut window.factory().borrow_mut();
                let encoder = &mut window.encoder().borrow_mut();
                let (width, height, _, _) = color.get_dimensions();
                let width = width as u32;
                let height = height as u32;

                // Schedule download.
                let download = factory
                    .create_download_buffer::<[u8; 4]>((width * height) as usize)
                    .unwrap();
                encoder
                    .copy_texture_to_buffer_raw(
                        color.raw().get_texture(),
                        None,
                        gfx::texture::RawImageInfo {
                            xoffset: 0,
                            yoffset: 0,
                            zoffset: 0,
                            width: width as u16,
                            height: height as u16,
                            depth: 0,
                            format: ColorFormat::get_format(),
                            mipmap: 0,
                        },
                        download.raw(),
                        0,
                    )
                    .unwrap();

                // Flush before reading the buffers to prevent panics.
                window.flush(encoder);

                // Copy to buffers.
                let mut pixels_rgb = Vec::with_capacity((width * height * 3) as usize);
                let reader = factory.read_mapping(&download).unwrap();
                for row in reader.chunks(width as usize).rev() {
                    for pixel in row.iter() {
                        pixels_rgb.push(pixel[0]);
                        pixels_rgb.push(pixel[1]);
                        pixels_rgb.push(pixel[2]);
                    }
                }

                let rgb_buffer = RgbBuffer {
                    pixels_rgb: pixels_rgb.into_boxed_slice(),
                    width,
                    height,
                };

                self.tx.send(Message::Buffer(rgb_buffer)).unwrap();
            }
            _ => {}
        }
    }
}
