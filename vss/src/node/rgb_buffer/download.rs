use wgpu::Buffer;

use super::*;
use std::io::Cursor;
use std::mem::size_of;
use std::sync::{mpsc, Arc, Mutex, OnceLock, RwLock};

pub type RgbBufferCb = Box<dyn FnOnce(RgbBuffer) + Send>;

type EncodeJob = Box<dyn FnOnce() + Send + 'static>;

fn encode_sender() -> &'static mpsc::SyncSender<EncodeJob> {
    static SENDER: OnceLock<mpsc::SyncSender<EncodeJob>> = OnceLock::new();
    SENDER.get_or_init(|| {
        let worker_count = std::thread::available_parallelism()
            .map(usize::from)
            .unwrap_or(1)
            .min(4);
        let (tx, rx) = mpsc::sync_channel::<EncodeJob>(worker_count * 2);
        let rx = Arc::new(Mutex::new(rx));
        for worker_index in 0..worker_count {
            let rx = rx.clone();
            std::thread::Builder::new()
                .name(format!("vss-image-encoder-{worker_index}"))
                .spawn(move || loop {
                    let Ok(job) = rx.lock().unwrap().recv() else {
                        break;
                    };
                    job();
                })
                .expect("failed to start image encoder worker");
        }
        tx
    })
}

fn enqueue_encode(job: EncodeJob) {
    if let Err(err) = encode_sender().send(job) {
        (err.0)();
    }
}

/// A node that downloads RGB buffers.
pub struct DownloadRgbBuffer {
    callback: Option<RgbBufferCb>,
    input: Texture,
    buffer: Buffer,
    res: [f32; 2],
}

impl DownloadRgbBuffer {
    pub fn new(context: &RenderContext) -> Self {
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
            callback: None,
            input: texture,
            buffer: download_buffer,
            res: [0.0, 0.0],
        }
    }

    pub fn set_buffer_cb(&mut self, cb: Option<RgbBufferCb>) {
        self.callback = cb;
    }

    pub fn set_image_encoder<F>(
        &mut self,
        format: crate::ImageFormat,
        write_output: F,
        processed: Arc<RwLock<bool>>,
        failure: Arc<RwLock<Option<String>>>,
        completion: mpsc::Sender<Result<(), String>>,
    ) where
        F: FnOnce(Vec<u8>) -> Result<(), String> + Send + 'static,
    {
        let cb = Box::new(move |rgb_buffer: RgbBuffer| {
            enqueue_encode(Box::new(move || {
                let result = (|| {
                    let img = image::RgbImage::from_raw(
                        rgb_buffer.width,
                        rgb_buffer.height,
                        rgb_buffer.pixels_rgb.into_vec(),
                    )
                    .ok_or_else(|| "failed to create RGB image buffer".to_string())?;
                    let mut encoded = Cursor::new(Vec::new());
                    image::DynamicImage::ImageRgb8(img)
                        .write_to(&mut encoded, format)
                        .map_err(|err| format!("failed to encode image: {err}"))?;
                    write_output(encoded.into_inner())
                })();

                match &result {
                    Ok(()) => *processed.write().unwrap() = true,
                    Err(message) => *failure.write().unwrap() = Some(message.clone()),
                }
                let _ = completion.send(result);
            }));
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
        let device = context.device();

        // Note that we're not calling `.await` here.
        let buffer_slice = self.buffer.slice(..);

        buffer_slice.map_async(wgpu::MapMode::Read, |_| {});

        let _ = device.poll(wgpu::PollType::wait_indefinitely());

        let buffer_dimensions =
            BufferDimensions::new(self.res[0] as usize, self.res[1] as usize, size_of::<u32>());
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
        if let Some(callback) = self.callback.take() {
            callback(rgb_buffer);
        }
    }
}
