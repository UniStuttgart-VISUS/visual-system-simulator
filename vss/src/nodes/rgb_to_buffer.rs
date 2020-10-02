use crate::pipeline::*;

pub type RgbBufferCb = Box<dyn FnOnce(RGBBuffer) + Send>;

enum Message {
    Buffer(RGBBuffer),
    Callback(Option<RgbBufferCb>),
}
/// A node that downloads RGB buffers.
pub struct RgbToBuffer {
    target: Option<DeviceTarget>,
    tx: std::sync::mpsc::Sender<Message>,
}

impl RgbToBuffer {
    pub fn set_output_cb(&mut self, cb: Option<RgbBufferCb>) {
        self.tx.send(Message::Callback(cb)).unwrap();
    }

    pub fn set_output_png(
        &mut self,
        path: String,
        processed: std::sync::Arc<std::sync::RwLock<bool>>,
    ) {
        let cb = Box::new(move |rgb_buffer: RGBBuffer| {
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
            println!("[image] written to {}", &path);
        });
        self.set_output_cb(Some(cb));
    }
}

impl Node for RgbToBuffer {
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

        RgbToBuffer { target: None, tx }
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
