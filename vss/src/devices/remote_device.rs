use std::cell::RefCell;
use std::io::Cursor;
use std::sync::mpsc;
use std::sync::{Arc, RwLock};
use std::thread;

use ws::{listen, CloseCode, Handler, Message, Result, Sender};

use super::*;
use crate::Config;

#[derive(Clone)]
enum RemoteMessage {
    Frame {
        name: String,
        width: u32,
        height: u32,
        rgba8: Vec<u8>,
    },
    //TODO config
}

/// A device that decorates another device with remote WebSocket capabilities.
pub struct RemoteDevice {
    publisher: mpsc::Sender<RemoteMessage>,
    clients: Arc<RwLock<Vec<RemoteClient>>>,
    device: Box<dyn Device>,
}

impl RemoteDevice {
    pub fn new(config: &Config, device: Box<dyn Device>) -> Self {
        let (tx, rx) = mpsc::channel();
        let device = RemoteDevice {
            publisher: tx,
            clients: Arc::new(RwLock::new(Vec::new())),
            device,
        };

        // Start listening thread.
        let clients = device.clients.clone();
        let port = config.port;
        thread::spawn(move || {
            println!("[remote] listening to ws://localhost:{}/", port);
            listen(format!("0.0.0.0:{}", port), |out| {
                println!("[remote] client connected");
                let client = RemoteClient {
                    out,
                    subscribed_frames: true,
                    msg: None,
                };
                clients.write().unwrap().push(client.clone());
                client
            })
            .unwrap()
        });

        // Start publishing thread.
        let clients = device.clients.clone();
        thread::spawn(move || loop {
            let subscriber_msg = rx.recv();
            if !subscriber_msg.is_ok() {
                break;
            }
            let subscriber_msg = subscriber_msg.unwrap();
            let clients = clients.read().unwrap();
            match subscriber_msg {
                RemoteMessage::Frame {
                    name: _,
                    width,
                    height,
                    rgba8,
                } => {
                    // Encode image.
                    let mut data = Vec::new();
                    let mut encoder = image::jpeg::JpegEncoder::new_with_quality(&mut data, 50);
                    encoder
                        .encode(&rgba8, width, height, image::ColorType::Rgba8)
                        .unwrap();
                    // Send to subscribed clients.
                    for client in clients.iter().filter(|client| client.subscribed_frames) {
                        client.out.send(data.clone()).unwrap();
                    }
                }
            }
        });

        device
    }

    pub fn send_frame(&mut self) {
        //TODO: something useful here.
        self.publisher
            .send(RemoteMessage::Frame {
                name: "".to_string(),
                width: 0,
                height: 0,
                rgba8: Vec::new(),
            })
            .unwrap();
    }
}

impl Device for RemoteDevice {
    fn factory(&self) -> &RefCell<DeviceFactory> {
        self.device.factory()
    }

    fn encoder(&self) -> &RefCell<DeviceEncoder> {
        self.device.encoder()
    }

    fn gaze(&self) -> DeviceGaze {
        self.device.gaze()
    }

    fn source(&self) -> &RefCell<DeviceSource> {
        self.device.source()
    }

    fn target(&self) -> &RefCell<DeviceTarget> {
        self.device.target()
    }

    fn begin_frame(&self) {
        self.device.begin_frame();
    }

    fn end_frame(&self, done: &mut bool) {
        self.device.end_frame(done);
    }
}

#[derive(Clone)]
struct RemoteClient {
    out: Sender,
    subscribed_frames: bool,
    msg: Option<RemoteMessage>,
}

impl RemoteClient {
    pub fn on_image(&mut self, text: &str) {
        self.msg = Some(RemoteMessage::Frame {
            name: text.to_string(),
            width: 0,
            height: 0,
            rgba8: Vec::new(),
        });
    }

    pub fn on_image_data(&mut self, data: &[u8]) {
        if let Some(RemoteMessage::Frame {
            name: _,
            ref mut width,
            ref mut height,
            ref mut rgba8,
        }) = self.msg
        {
            match image::load(Cursor::new(data), image::ImageFormat::Png) {
                Ok(img) => {
                    let img = img.flipv().to_rgba();
                    let (img_w, img_h) = img.dimensions();
                    *width = img_w as u32;
                    *height = img_h as u32;
                    *rgba8 = img.into_raw().to_vec();
                }
                Err(_e) => {
                    println!("[remote] dropping bad image data");
                }
            }
        } else {
            println!("[remote] dropping unexpected image data");
        }
    }
}

impl Handler for RemoteClient {
    fn on_message(&mut self, msg: Message) -> Result<()> {
        match msg {
            Message::Text(ref text) => {
                let text = text.trim();
                if text.starts_with('{') {
                    // JSON syntax implies config.
                    // TODO: ...
                    let _value: serde_json::Value = serde_json::from_str(text).unwrap();
                } else if text == "PAUSE" {
                    // Stop publishing frames to this client.
                    self.subscribed_frames = false;
                } else if text != "CONTINUE" {
                    // Continue publishing frames to this client.
                    self.subscribed_frames = true;
                } else {
                    // Otherwise we assume an image name (client will send an image next).
                    self.on_image(text);
                }
                Ok(())
            }
            Message::Binary(data) => {
                self.on_image_data(&data);
                Ok(())
            }
        }
    }

    fn on_close(&mut self, code: CloseCode, reason: &str) {
        println!("[remote] closing with {:?}: {}", code, reason);
    }
}
