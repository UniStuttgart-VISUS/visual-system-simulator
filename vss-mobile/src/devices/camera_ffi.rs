#[cfg(target_os="android")]


#[cfg(target_os = "android")]
pub mod camera_ffi;


#[cfg(target_os = "android")]
pub use self::camera_ffi::Java_de_uni_1stuttgart_vis_fist_activities_CoreActivity_postData;

#[cfg(target_os = "android")]
pub use self::camera_ffi::Java_de_uni_1stuttgart_vis_fist_activities_CoreActivity_postConfig;


use std::ptr;

use jni::JNIEnv;
use jni::objects::JClass;
use jni::sys::jstring;
use jni::objects::JString;
use jni::sys::jint;
use jni::sys::jbyteArray;
use std::cell::RefCell;

use android_glue;

use std::{thread, time};

use std::sync::mpsc::Receiver;
use std::sync::mpsc::SyncSender;
use std::sync::mpsc;
use std::sync::Mutex;

use std::io::Write;

use config::ED_CONFIG;
use config::ED_CONFIG_UPDATE_FLAG;

use std::time::{Duration, Instant};



lazy_static!{
    pub static ref channel : (Mutex<SyncSender<Frame>>,Mutex<Receiver<Frame>>)= {
        let (a,b) = mpsc::sync_channel(2);
        (Mutex::new(a),Mutex::new(b))
    };

    pub static ref preview_channel : (Mutex<SyncSender<Frame>>,Mutex<Receiver<Frame>>)= {
        let (a,b) = mpsc::sync_channel(2);
        (Mutex::new(a),Mutex::new(b))
    };

//pub static ref config : Mutex<RefCell<String>>= {
//       Mutex::new(RefCell::new(String::from("{}")))
//    };
}

static mut ctr :u64 = 0;

pub struct Frame{
    pub y: Box<[u8]>,
    pub u: Box<[u8]>,
    pub v: Box<[u8]>,
    pub width: u32,
    pub height: u32,
}

#[inline(never)]
#[allow(non_snake_case)]
#[no_mangle]
pub unsafe extern "C" fn Java_de_uni_1stuttgart_vis_fist_activities_CoreActivity_postData(env: JNIEnv,
                                                                      _: JClass,
                                                                      width: jint,
                                                                      height: jint,
                                                                      y: jbyteArray,
                                                                      u: jbyteArray,
                                                                      v: jbyteArray
) {


    let mutex = &channel.0;
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
}

#[inline(never)]
#[allow(non_snake_case)]
#[no_mangle]
pub unsafe extern "C" fn Java_de_uni_1stuttgart_vis_fist_activities_CoreActivity_postConfig(env: JNIEnv,
                                                                                 _: JClass,
                                                                                 conf: JString
) {
    let conf: String = env.get_string(conf).expect("Couldn't get java string!").into();
    let mut s = &ED_CONFIG.lock().unwrap();
    let mut s = s.borrow_mut();
    s.clear();
    s.push_str(conf.as_str());

    let flag = &ED_CONFIG_UPDATE_FLAG.lock().unwrap();
    let mut flag = flag.borrow_mut();
    *flag = true;

    println!("RustConfReceiver: {}",conf);
}

