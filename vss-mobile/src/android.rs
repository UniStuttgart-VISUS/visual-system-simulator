#![allow(non_snake_case)]
#![cfg(target_os = "android")]

use std::cell::RefCell;
use std::ffi::c_void;
use std::panic;
use std::ptr::NonNull;
use std::sync::{mpsc, Mutex, MutexGuard};

use futures::executor::block_on;

use log::*;

use jni::objects::{JClass, JObject, JString};
use jni::sys::{jbyteArray, jint};
use jni::JNIEnv;

use ndk_sys;

use android_logger::{Config, FilterBuilder};

use raw_window_handle::*;

use vss::*;

enum Message {
    Config(vss::ValueMap),
    Frame(vss::RgbBuffer), //TODO: should be a YUV buffer
}

struct AndroidHandle(RawWindowHandle);

unsafe impl HasRawWindowHandle for AndroidHandle {
    fn raw_window_handle(&self) -> RawWindowHandle {
        self.0
    }
}

unsafe impl HasRawDisplayHandle for AndroidHandle {
    fn raw_display_handle(&self) -> RawDisplayHandle {
        RawDisplayHandle::Android(AndroidDisplayHandle::empty())
    }
}

struct Bridge {
    pub surface: Surface,
}

unsafe impl Send for Bridge {}

lazy_static::lazy_static! {
    static ref BRIDGE : Mutex<Option<Bridge>> = Mutex::new(None);
}

#[no_mangle]
pub extern "system" fn Java_com_vss_simulator_SimulatorBridge_nativeCreate(
    env: JNIEnv,
    _class: JClass,
    surface: JObject,
    assetManager: JObject,
) {
    android_logger::init_once(
        Config::default()
            .with_max_level(LevelFilter::Trace)
            .with_tag("VSS"),
    );

    panic::set_hook(Box::new(|info| {
        error!("{}", info.to_string());
    }));

    let mut guard: MutexGuard<'_, Option<Bridge>> = BRIDGE.lock().unwrap();

    let window = unsafe {
        ndk::native_window::NativeWindow::from_ptr(
            NonNull::new(ndk_sys::ANativeWindow_fromSurface(
                env.get_raw(),
                surface.as_raw(),
            ))
            .unwrap(),
        )
    };
    let assetManager = unsafe {
        ndk::asset::AssetManager::from_ptr(
            NonNull::new(ndk_sys::AAssetManager_fromJava(
                env.get_raw(),
                assetManager.as_raw(),
            ))
            .unwrap(),
        );
    };

    let mut value_map = ValueMap::new();
    let mut parameters: Vec<RefCell<ValueMap>> = vec![RefCell::new(value_map)];
    let mut window_handle = AndroidNdkWindowHandle::empty();
    window_handle.a_native_window = window.ptr().as_ptr() as *mut c_void;
    let handle = AndroidHandle(RawWindowHandle::AndroidNdk(window_handle));
    let size = [window.width() as u32, window.height() as u32];
    let surface = vss::Surface::new(size, handle, None, parameters, 1);
    let surface = futures::executor::block_on(surface);

    *guard = Some(Bridge { surface });
}

#[no_mangle]
pub extern "system" fn Java_com_vss_simulator_SimulatorBridge_nativeDestroy(
    env: JNIEnv,
    _class: JClass,
) {
    let mut guard: MutexGuard<'_, Option<Bridge>> = BRIDGE.lock().unwrap();
    *guard = None;
}

#[no_mangle]
pub extern "system" fn Java_com_vss_simulator_SimulatorBridge_nativeResize(
    env: JNIEnv,
    _class: JClass,
    width: jint,
    height: jint,
) {
    let mut guard: MutexGuard<'_, Option<Bridge>> = BRIDGE.lock().unwrap();
    if let Some(ref mut bridge) = *guard {
        bridge.surface.resize([width as u32, height as u32])
    }
}

#[no_mangle]
pub extern "system" fn Java_com_vss_simulator_SimulatorBridge_nativeDraw(
    env: JNIEnv,
    _class: JClass,
) {
    let mut guard: MutexGuard<'_, Option<Bridge>> = BRIDGE.lock().unwrap();
    if let Some(ref bridge) = *guard {
        //TODO: bridge.render_the_flow()
    }
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
    let mut guard: MutexGuard<'_, Option<Bridge>> = BRIDGE.lock().unwrap();

    if let Some(ref bridge) = *guard {
        /*
        let y = env.convert_byte_array(y).unwrap().into_boxed_slice();
        let u = env.convert_byte_array(u).unwrap().into_boxed_slice();
        let v = env.convert_byte_array(v).unwrap().into_boxed_slice();

        let frame = Frame {
            y, u, v,
            width:width as u32,
            height:height as u32,
        };

        //TODO: bridge.post_frame()
        */
    }
}

#[no_mangle]
pub extern "system" fn Java_com_vss_simulator_SimulatorBridge_nativePostSettings(
    env: JNIEnv,
    _class: JClass,
    json_string: JString,
) {
    let mut guard: MutexGuard<'_, Option<Bridge>> = BRIDGE.lock().unwrap();

    //let json_string: String = env.get_string(son_string).expect("Java String expected").into();

    if let Some(ref bridge) = *guard {
        //TODO: bridge.post_settings()
    }
}
