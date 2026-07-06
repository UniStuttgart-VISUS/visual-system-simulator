use wgpu::Buffer;

use super::*;
use std::io::Cursor;
use std::mem::size_of;
use std::sync::{Arc, RwLock};

pub type RgbBufferCb = Box<dyn FnOnce(RgbBuffer) + Send>;

enum Message {
    Buffer(RgbBuffer),
    Callback(Option<RgbBufferCb>),
}
/// A node that downloads RGB buffers.
pub struct DownloadRgbBuffer {
    tx: std::sync::mpsc::Sender<Message>,
    input: Texture,
    buffer: Buffer,
    res: [f32; 2],
}

impl DownloadRgbBuffer {
    pub fn new(context: &RenderContext) -> Self {
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

        let device = context.device();
        let queue = context.queue();

        let buffer_dimensions = BufferDimensions::new(1, 1, size_of::<u32>());

        let download_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Download Node Placeholder Buffer"),
            size: (buffer_dimensions.padded_bytes_per_row * buffer_dimensions.height) as u64,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let texture =
            placeholder_texture(device, queue, Some("Download texture placeholder")).unwrap();

        DownloadRgbBuffer {
            tx,
            input: texture,
            buffer: download_buffer,
            res: [0.0, 0.0],
        }
    }

    pub fn set_buffer_cb(&mut self, cb: Option<RgbBufferCb>) {
        self.tx.send(Message::Callback(cb)).unwrap();
    }

    pub fn set_image_encoder<F>(
        &mut self,
        format: crate::ImageFormat,
        write_output: F,
        processed: Arc<RwLock<bool>>,
        failure: Arc<RwLock<Option<String>>>,
    ) where
        F: FnOnce(Vec<u8>) -> Result<(), String> + Send + 'static,
    {
        let cb = Box::new(move |rgb_buffer: RgbBuffer| {
            let Some(img) = image::RgbImage::from_raw(
                rgb_buffer.width,
                rgb_buffer.height,
                rgb_buffer.pixels_rgb.into_vec(),
            ) else {
                *failure.write().unwrap() = Some("failed to create RGB image buffer".to_string());
                return;
            };

            let mut encoded = Cursor::new(Vec::new());
            if let Err(err) = image::DynamicImage::ImageRgb8(img).write_to(&mut encoded, format) {
                *failure.write().unwrap() = Some(format!("failed to encode image: {err}"));
                return;
            }

            if let Err(err) = write_output(encoded.into_inner()) {
                *failure.write().unwrap() = Some(err);
                return;
            }

            *processed.write().unwrap() = true;
        });
        self.set_buffer_cb(Some(cb));
    }
}

impl Node for DownloadRgbBuffer {
    fn name(&self) -> &'static str {
        "DownloadRgbBuffer"
    }

    fn negociate_slots(
        &mut self,
        context: &RenderContext,
        slots: NodeSlots,
        _original_image: &mut Option<Texture>,
    ) -> NodeSlots {
        let slots = slots.to_color_input(context);
        self.res = slots.input_size_f32();
        let device = context.device();

        (self.input, _) = slots.as_color_source(device);

        let buffer_dimensions =
            BufferDimensions::new(self.res[0] as usize, self.res[1] as usize, size_of::<u32>());
        println!(
            "negociate_slots {}, {}",
            buffer_dimensions.width, buffer_dimensions.height
        );
        let download_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Download Node Buffer"),
            size: (buffer_dimensions.padded_bytes_per_row * buffer_dimensions.height) as u64,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        self.buffer = download_buffer;

        slots
    }

    fn render(
        &mut self,
        _context: &RenderContext,
        encoder: &mut CommandEncoder,
        _screen: Option<&RenderTexture>,
    ) {
        let buffer_dimensions =
            BufferDimensions::new(self.res[0] as usize, self.res[1] as usize, size_of::<u32>());
        println!(
            "render {}, {}",
            buffer_dimensions.width, buffer_dimensions.height
        );

        let texture_extent = wgpu::Extent3d {
            width: buffer_dimensions.width as u32,
            height: buffer_dimensions.height as u32,
            depth_or_array_layers: 1,
        };

        // Schedule download.
        encoder.copy_texture_to_buffer(
            self.input.texture.as_image_copy(),
            wgpu::TexelCopyBufferInfo {
                buffer: &self.buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(buffer_dimensions.padded_bytes_per_row as u32),
                    rows_per_image: None,
                },
            },
            texture_extent,
        );
    }

    fn post_render(&mut self, context: &RenderContext) {
        println!("download post_render");
        let device = context.device();

        // Note that we're not calling `.await` here.
        let buffer_slice = self.buffer.slice(..);

        let (sender, _receiver) = futures_intrusive::channel::shared::oneshot_channel();
        buffer_slice.map_async(wgpu::MapMode::Read, move |v| {
            sender.send(v).unwrap();
            println!("sender ok");
        });

        let _ = device.poll(wgpu::PollType::wait_indefinitely());

        let buffer_dimensions =
            BufferDimensions::new(self.res[0] as usize, self.res[1] as usize, size_of::<u32>());
        println!(
            "post_render {}, {}",
            buffer_dimensions.width, buffer_dimensions.height
        );
        let padded_buffer = buffer_slice.get_mapped_range();

        let mut pixels_rgb =
            Vec::with_capacity(buffer_dimensions.width * buffer_dimensions.height * 3);
        // from the padded_buffer we write just the unpadded bytes into the image
        for chunk in padded_buffer.chunks(buffer_dimensions.padded_bytes_per_row) {
            for i in (0..buffer_dimensions.unpadded_bytes_per_row).step_by(4) {
                pixels_rgb.push(chunk[i]);
                pixels_rgb.push(chunk[i + 1]);
                pixels_rgb.push(chunk[i + 2]);
            }
        }

        let rgb_buffer = RgbBuffer {
            pixels_rgb: pixels_rgb.into_boxed_slice(),
            width: buffer_dimensions.width as u32,
            height: buffer_dimensions.height as u32,
        };
        drop(padded_buffer);
        self.buffer.unmap();
        self.tx.send(Message::Buffer(rgb_buffer)).unwrap();
    }
}
