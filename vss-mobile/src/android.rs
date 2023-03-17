#![allow(non_snake_case)]
#![cfg(target_os = "android")]

use std::cell::RefCell;
use std::ffi::c_void;
use std::panic;
use std::ptr::NonNull;
use std::sync::{Mutex, MutexGuard};

use log::*;

use jni::objects::{JByteArray, JClass, JObject, JString};
use jni::sys::jint;
use jni::JNIEnv;

use ndk_sys;

use android_logger::Config;

use raw_window_handle::*;

use vss::*;

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
pub extern "system" fn Java_com_vss_simulator_SimulatorBridge_nativeCreate<'local>(
    env: JNIEnv<'local>,
    _class: JClass,
    surface: JObject<'local>,
    assetManager: JObject<'local>,
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

    let value_map = ValueMap::new();
    let parameters: Vec<RefCell<ValueMap>> = vec![RefCell::new(value_map)];
    let mut window_handle = AndroidNdkWindowHandle::empty();
    window_handle.a_native_window = window.ptr().as_ptr() as *mut c_void;
    let handle = AndroidHandle(RawWindowHandle::AndroidNdk(window_handle));
    let size = [window.width() as u32, window.height() as u32];
    let surface = vss::Surface::new(size, handle, None, parameters, 1);
    let mut surface = futures::executor::block_on(surface);

    build_flow(&mut surface);
    surface.update_nodes();

    *guard = Some(Bridge { surface });
}

fn build_flow(surface: &mut Surface) {
    //TODO: use a proper set of nodes.

    let buffer = RgbBuffer {
        pixels_rgb: Box::new([127; 16 * 16 * 4]),
        width: 16,
        height: 16,
    };
    let mut node = UploadRgbBuffer::new(&surface);
    node.upload_buffer(&buffer);
    surface.add_node(Box::new(node), 0);

    let node = Display::new(&surface);
    surface.add_node(Box::new(node), 0);
}

#[no_mangle]
pub extern "system" fn Java_com_vss_simulator_SimulatorBridge_nativeDestroy<'local>(
    _env: JNIEnv<'local>,
    _class: JClass,
) {
    let mut guard: MutexGuard<'_, Option<Bridge>> = BRIDGE.lock().unwrap();
    *guard = None;
}

#[no_mangle]
pub extern "system" fn Java_com_vss_simulator_SimulatorBridge_nativeResize<'local>(
    _env: JNIEnv<'local>,
    _class: JClass,
    width: jint,
    height: jint,
) {
    let mut guard: MutexGuard<'_, Option<Bridge>> = BRIDGE.lock().unwrap();
    let bridge = (*guard).as_mut().expect("Bridge should be created");
    bridge.surface.resize([width as u32, height as u32]);
}

#[no_mangle]
pub extern "system" fn Java_com_vss_simulator_SimulatorBridge_nativeDraw<'local>(
    _env: JNIEnv<'local>,
    _class: JClass,
) {
    let mut guard: MutexGuard<'_, Option<Bridge>> = BRIDGE.lock().unwrap();
    let bridge = (*guard).as_mut().expect("Bridge should be created");
    bridge.surface.draw();
}

#[no_mangle]
pub extern "system" fn Java_com_vss_simulator_SimulatorBridge_nativePostFrame<'local>(
    env: JNIEnv<'local>,
    _class: JClass,
    width: jint,
    height: jint,
    y: JByteArray<'local>,
    u: JByteArray<'local>,
    v: JByteArray<'local>,
) {
    let mut guard: MutexGuard<'_, Option<Bridge>> = BRIDGE.lock().unwrap();
    let bridge = (*guard).as_mut().expect("Bridge should be created");

    let pixels_y = env.convert_byte_array(&y).unwrap().into_boxed_slice();
    let pixels_u = env.convert_byte_array(&u).unwrap().into_boxed_slice();
    let pixels_v = env.convert_byte_array(&v).unwrap().into_boxed_slice();

    let buffer = YuvBuffer {
        pixels_y,
        pixels_u,
        pixels_v,
        width: width as u32,
        height: height as u32,
    };

    //TODO: bridge.post_frame()
}

#[no_mangle]
pub extern "system" fn Java_com_vss_simulator_SimulatorBridge_nativePostSettings<'local>(
    mut env: JNIEnv<'local>,
    _class: JClass,
    json_string: JString<'local>,
) {
    let mut guard: MutexGuard<'_, Option<Bridge>> = BRIDGE.lock().unwrap();
    let bridge = (*guard).as_mut().expect("Bridge should be created");

    let json_string: String = env
        .get_string(&json_string)
        .expect("Should be a Java String")
        .into();

    //TODO: bridge.post_settings()
}
