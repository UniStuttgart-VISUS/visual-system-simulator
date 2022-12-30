use wgpu::{ImageCopyTexture};

use super::*;
use std::{path::Path, mem::size_of};

pub type RgbBufferCb = Box<dyn FnOnce(RgbBuffer) + Send>;

// from https://github.com/gfx-rs/wgpu/blob/6b6bc69ba07675697dfbadcf7ba5b035f5dfe5f7/wgpu/examples/capture/main.rs
struct BufferDimensions {
    width: usize,
    height: usize,
    unpadded_bytes_per_row: usize,
    padded_bytes_per_row: usize,
}

impl BufferDimensions {
    fn new(width: usize, height: usize) -> Self {
        let bytes_per_pixel = size_of::<u32>();
        let unpadded_bytes_per_row = width * bytes_per_pixel;
        let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT as usize;
        let padded_bytes_per_row_padding = (align - unpadded_bytes_per_row % align) % align;
        let padded_bytes_per_row = unpadded_bytes_per_row + padded_bytes_per_row_padding;
        Self {
            width,
            height,
            unpadded_bytes_per_row,
            padded_bytes_per_row,
        }
    }
}

enum Message {
    Buffer(RgbBuffer),
    Callback(Option<RgbBufferCb>),
}
/// A node that downloads RGB buffers.
pub struct DownloadRgbBuffer<'a> {
    tx: std::sync::mpsc::Sender<Message>,
    input: Option<RenderTexture>,
    buffer_slice: Option<wgpu::BufferSlice<'a>>, 
    res: [f32;2]
}

impl DownloadRgbBuffer<'_> {
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

impl Node for DownloadRgbBuffer<'_> {
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
            tx,
            input: None,
            buffer_slice: None,
            res: [0.0,0.0]
        }
    }

    fn negociate_slots(&mut self, window: &Window, slots: NodeSlots) -> NodeSlots {
        let slots = slots.to_color_input(window);
        self.res = slots.input_size_f32();
        self.input = Some(slots.as_color_target());

        slots
    }

    fn render(&mut self, window: &window::Window, encoder: &mut CommandEncoder, _screen: Option<&RenderTexture>) {
        match &self.input {
            Some(_) => {

                let device = window.device().borrow_mut();

                let buffer_dimensions = BufferDimensions::new(self.res[0] as usize, self.res[1] as usize);

                let download_buffer = device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("Download Node Buffer"),
                    size: (buffer_dimensions.padded_bytes_per_row * buffer_dimensions.height) as u64,
                    usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
                    mapped_at_creation: false,
                });
            
                let texture_extent = wgpu::Extent3d {
                    width: buffer_dimensions.width as u32,
                    height: buffer_dimensions.height as u32,
                    depth_or_array_layers: 1,
                };

                // Schedule download.
                encoder.copy_texture_to_buffer(
                    ImageCopyTexture{
                            texture: self.input.as_ref().unwrap().texture.as_ref().unwrap(),
                            aspect: wgpu::TextureAspect::All,
                            mip_level: 0,
                            origin: wgpu::Origin3d::ZERO,
                        },
                    wgpu::ImageCopyBuffer {
                        buffer: &download_buffer,
                        layout: wgpu::ImageDataLayout {
                            offset: 0,
                            bytes_per_row: Some(
                                std::num::NonZeroU32::new(buffer_dimensions.padded_bytes_per_row as u32)
                                    .unwrap(),
                            ),
                            rows_per_image: None,
                        },
                    },
                    texture_extent,
                );

                // Note that we're not calling `.await` here.
                let buffer_slice = download_buffer.slice(..);
                
                let (sender, _receiver) = futures_intrusive::channel::shared::oneshot_channel();
                buffer_slice.map_async(wgpu::MapMode::Read, move |v| sender.send(v).unwrap());
                
            }
            _ => {}
        }
    }

    fn post_render(&mut self, _window: &window::Window) {
        match self.buffer_slice {
            Some(_) => {
                let buffer_dimensions = BufferDimensions::new(self.res[0] as usize, self.res[1] as usize);
                let padded_buffer = self.buffer_slice.unwrap().get_mapped_range();

                let mut pixels_rgb = Vec::with_capacity(buffer_dimensions.width * buffer_dimensions.height * 3);
                // from the padded_buffer we write just the unpadded bytes into the image
                for chunk in padded_buffer.chunks(buffer_dimensions.padded_bytes_per_row) {
                    for i in (0 .. buffer_dimensions.unpadded_bytes_per_row).step_by(4) {
                        pixels_rgb.push(chunk[i]);
                        pixels_rgb.push(chunk[i+1]);
                        pixels_rgb.push(chunk[i+2]);
                    }
                }
        
                let rgb_buffer = RgbBuffer {
                    pixels_rgb: pixels_rgb.into_boxed_slice(),
                    width: buffer_dimensions.width as u32,
                    height: buffer_dimensions.height as u32,
                };
        
                self.tx.send(Message::Buffer(rgb_buffer)).unwrap();
            },
            None => {},
        }
    }
}
