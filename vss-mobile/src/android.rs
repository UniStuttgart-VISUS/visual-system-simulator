#![allow(non_snake_case)]
#![cfg(target_os = "android")]

use std::sync::{mpsc, Mutex};

use jni::objects::{JClass, JObject, JString};
use jni::sys::{jbyteArray, jint};
use jni::JNIEnv;

use android_activity::AndroidApp;

pub enum Message {
    Config(String),
    Frame {
        y: Box<[u8]>,
        u: Box<[u8]>,
        v: Box<[u8]>,
        width: u32,
        height: u32,
    },
}

lazy_static::lazy_static! {
    pub static ref MESSAGE_QUEUE : (Mutex<mpsc::SyncSender<Message>>,Mutex<mpsc::Receiver<Message>>)= {
        // Create a message queue that blocks the sender when the queue is full.
        let (tx,rx) = mpsc::sync_channel(2);
        (Mutex::new(tx),Mutex::new(rx))
    };
}

/// Program entry point for Android.
#[no_mangle]
fn android_main(_app: AndroidApp) {
    println!("Hello World");
    /*
    let mut config = config::get_default_config();
    config.input_file = String::from("flowers.png");
    config.loop_provider = String::from("camera");
    config.special_loop_provider_set = true;


    let (mut device, mut flow) = config.build();

    let mut done = false;
    while !done {
        device.begin_frame();
        flow.render(&mut device);
        device.end_frame(&mut done);
    }
    */
}

#[no_mangle]
pub extern "system" fn Java_com_vss_activities_MainActivity_postConfig(
    env: JNIEnv,
    _class: JClass,
    json_string: JString,
) {
    println!("Config!");

    /*
      let conf: String = env.get_string(conf).expect("Couldn't get java string!").into();
       let mut s = &ED_CONFIG.lock().unwrap();
       let mut s = s.borrow_mut();
       s.clear();
       s.push_str(conf.as_str());

       let flag = &ED_CONFIG_UPDATE_FLAG.lock().unwrap();
       let mut flag = flag.borrow_mut();
       *flag = true;

       println!("RustConfReceiver: {}",conf);
    */
}

#[no_mangle]
pub extern "system" fn Java_com_vss_activities_MainActivity_postFrame(
    env: JNIEnv,
    _class: JClass,
    width: jint,
    height: jint,
    y: jbyteArray,
    u: jbyteArray,
    v: jbyteArray,
) {
    println!("Frame!");

    /*

    let mutex = &event_queue.0;
    let sender = mutex.lock().unwrap();

    {
        let ya = env.convert_byte_array(y).unwrap();
        let ua = env.convert_byte_array(u).unwrap();
        let va = env.convert_byte_array(v).unwrap();

        let frame = Frame {
            y: ya.into_boxed_slice(),
            u: ua.into_boxed_slice(),
            v: va.into_boxed_slice(),
            width:width as u32,
            height:height as u32,
        };

        let res = sender.try_send(frame);

        if res.is_err() {
            println!("SendError: {} ", res.err().unwrap());
        }

    }
     */
}
