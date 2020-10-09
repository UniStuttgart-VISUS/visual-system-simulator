use crate::pipeline::*;
use std::path::Path;

pub type RgbBufferCb = Box<dyn FnOnce(RgbBuffer) + Send>;

enum Message {
    Buffer(RgbBuffer),
    Callback(Option<RgbBufferCb>),
}
/// A node that downloads RGB buffers.
pub struct DownloadRgbBuffer {
    target: Option<DeviceTarget>,
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

        DownloadRgbBuffer { target: None, tx }
    }

    fn update_io(
        &mut self,
        _window: &Window,
        source: (Option<DeviceSource>, Option<DeviceTarget>),
        _target_candidate: (Option<DeviceSource>, Option<DeviceTarget>),
    ) -> (Option<DeviceSource>, Option<DeviceTarget>) {
        self.target = source.1.clone();
        source
    }

    fn render(&mut self, window: &Window) {
        if let Some(target) = &self.target {
            let rgb_buffer = download_rgb(window, target);
            self.tx.send(Message::Buffer(rgb_buffer)).unwrap();
        }
    }
}
