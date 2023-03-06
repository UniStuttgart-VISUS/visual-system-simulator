#![allow(non_snake_case)]
#![cfg(target_os = "android")]

use std::sync::{mpsc, Mutex};

use jni::objects::{JClass, JObject, JString};
use jni::sys::{jbyteArray, jint};
use jni::JNIEnv;

/* 
use std::{os::raw::c_void, panic::catch_unwind, sync::mpsc, thread, time::Duration};
use jni::{JNIEnv, JavaVM};
use jni::objects::{GlobalRef, JClass, JObject, JString};
use jni::sys::{jbyteArray, jint, jlong, jstring, JNI_VERSION_1_6};
*/

pub enum Message {
    Config(vss::ValueMap),
    Frame(vss::RgbBuffer)  , //TODO: should be a YUV buffer
}

lazy_static::lazy_static! {
    pub static ref MESSAGE_QUEUE : (Mutex<mpsc::SyncSender<Message>>,Mutex<mpsc::Receiver<Message>>)= {
        // Create a message queue that blocks the sender when the queue is full.
        let (tx,rx) = mpsc::sync_channel(2);
        (Mutex::new(tx),Mutex::new(rx))
    };
}

#[no_mangle]
pub extern "system" fn  Java_com_vss_simulator_SimulatorBridge_nativeCreate(
    env: JNIEnv,
    _class: JClass,
      surface: JObject, 
        assetManager : JObject,
) {
    println!("Create!");
}

#[no_mangle]
pub extern "system" fn  Java_com_vss_simulator_SimulatorBridge_nativeDestroy(
    env: JNIEnv,
    _class: JClass,
) {
    println!("Destroy!");
}

#[no_mangle]
pub extern "system" fn  Java_com_vss_simulator_SimulatorBridge_nativeResize(
    env: JNIEnv,
    _class: JClass,
    width: jint,
    height: jint,
) {
    println!("Resize!");
}
 
 
#[no_mangle]
pub extern "system" fn Java_com_vss_simulator_SimulatorBridge_nativeDraw(
    env: JNIEnv,
    _class: JClass, 
) {
    println!("Draw!");
}

#[no_mangle]
pub extern "system" fn Java_com_vss_simulator_SimulatorBridge_nativePostFrame(
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

#[no_mangle]
pub extern "system" fn Java_com_vss_simulator_SimulatorBridge_nativePostSettings(
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
