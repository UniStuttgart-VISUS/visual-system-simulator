#![allow(non_snake_case)]
#![cfg(target_os = "android")]

use std::cell::RefCell;
use std::ffi::c_void;
use std::panic;
use std::ptr::NonNull;
use std::sync::{Mutex, MutexGuard};
use std::sync::mpsc::{self, Receiver, SyncSender};

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

struct CameraStream {
    upload: UploadYuvBuffer,
    frame_receiver: Receiver<YuvBuffer>,
}

impl CameraStream {
    fn new(surface: &Surface, frame_receiver: Receiver<YuvBuffer>) -> Self {
        let mut upload = UploadYuvBuffer::new(&surface);
        upload.set_format(YuvFormat::_420888);
        CameraStream {
            upload,
            frame_receiver,
        }
    }
}

impl Node for CameraStream {
    fn negociate_slots(&mut self, surface: &Surface, slots: NodeSlots, original_image: &mut Option<Texture>) -> NodeSlots {
        self.upload.negociate_slots(&surface, slots, original_image)
    }

    fn inspect(&mut self, surface: &Surface, inspector: &mut dyn Inspector) {
        self.upload.inspect(&surface, inspector);
    }

    fn input(
        &mut self,
        perspective: &EyePerspective,
        vis_param: &VisualizationParameters,
    ) -> EyePerspective {
        let buffer = self.frame_receiver.try_recv();
        if let Ok(buffer) = buffer {
            debug!("Uploading... {}x{} {}", buffer.width, buffer.height, buffer.pixels_y[0]);
            self.upload.upload_buffer(buffer);
        }
        self.upload.input(&perspective, vis_param)
    }

    fn render(
        &mut self,
        surface: &Surface,
        encoder: &mut wgpu::CommandEncoder,
        screen: Option<&RenderTexture>,
    ) {
        self.upload.render(&surface, encoder, screen);
    }

    fn post_render(&mut self, surface: &Surface) {
        self.upload.post_render(&surface);
    }
}

struct Bridge {
    pub surface: Surface,
    pub frame_sender: SyncSender<YuvBuffer>,
    pub current_size: [i32; 2],
    pub new_size: [i32; 2],
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
            .with_tag("libvss"),
    );

    panic::set_hook(Box::new(|info| {
        error!("{}", info.to_string());
    }));

    info!(
        "Logger setup complete ({})",
        if cfg!(debug_assertions) {
            "debug"
        } else {
            "release"
        }
    );

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

    // let mut value_map = ValueMap::new();

    //TODO for testing purposes only
    // value_map.insert("peacock_cb_onoff".into(), Value::Bool(true));
    // value_map.insert("peacock_cb_strength".into(), Value::Number(1.0 as f64));
    // value_map.insert("peacock_cb_type".into(), Value::Number(0.0 as f64));
    // value_map.insert("colorblindness_onoff".into(), Value::Bool(true));
    // value_map.insert("colorblindness_type".into(), Value::Number(0.0 as f64));
    // value_map.insert("colorblindness_int".into(), Value::Number(100.0 as f64));
    // value_map.insert("cubemap_scale".into(), Value::Number(0.1 as f64));

    // let parameters: Vec<RefCell<ValueMap>> = vec![RefCell::new(value_map)];
    let mut window_handle = AndroidNdkWindowHandle::empty();
    window_handle.a_native_window = window.ptr().as_ptr() as *mut c_void;
    let handle = AndroidHandle(RawWindowHandle::AndroidNdk(window_handle));
    let size = [window.width() as u32, window.height() as u32];
    let surface = vss::Surface::new(size, handle, 1, None);
    let mut surface = futures::executor::block_on(surface);

    let (tx, rx) = mpsc::sync_channel(2);
    build_flow(&mut surface, rx);
    //TODO: surface.inspect();

    
    *guard = Some(Bridge { 
        surface,
        frame_sender: tx,
        current_size: [1, 1],
        new_size: [1, 1],
     });
}

fn build_flow(surface: &mut Surface, frame_receiver: Receiver<YuvBuffer>) {
    //TODO: use a proper set of nodes.

    // Add input node.
    let node = CameraStream::new(&surface, frame_receiver);
    surface.add_node(Box::new(node), 0);

    // Visual system passes.
    // let node = Lens::new(&surface);
    // surface.add_node(Box::new(node), 0);
    // let node = Cataract::new(&surface);
    // surface.add_node(Box::new(node), 0);
    let node = Retina::new(&surface);
    surface.add_node(Box::new(node), 0);
    // let node = PeacockCB::new(&surface);
    // surface.add_node(Box::new(node), 0);

    // Display node.
    let mut node = Display::new(&surface);
    node.set_output_scale(OutputScale::Fill);
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
    for f in bridge.surface.flow.iter(){
        f.input(&bridge.surface.vis_param.borrow());
    }
    //TODO replace this with some better way of triggering a node update
    //(it is neccessary to refresh node resolutions but for this we need the upload node to have a buffer available to get the new resolution from)
    if (bridge.new_size[0] != bridge.current_size[0]) || (bridge.new_size[1] != bridge.current_size[1]) {
        debug!("Buffer sizes do not match, old({}, {}), new({}, {})", bridge.current_size[0], bridge.current_size[1], bridge.new_size[0], bridge.new_size[1]);
        //TODO: bridge.surface.inspect();
        bridge.current_size = bridge.new_size;
    }
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

    let res = bridge.frame_sender.try_send(buffer);
    if res.is_err() {
        warn!("{}, dropping frame", res.err().unwrap());
    }else{
        bridge.new_size = [width, height];
    }
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

#[no_mangle]
pub extern "system" fn Java_com_vss_simulator_SimulatorBridge_nativeQuerySettings<'local>(
    mut env: JNIEnv<'local>,
    _class: JClass
) -> JString<'local> {
    let mut guard: MutexGuard<'_, Option<Bridge>> = BRIDGE.lock().unwrap();
    let bridge = (*guard).as_mut().expect("Bridge should be created");

    //TODO: bridge.query_settings()

    return env.new_string("{}").unwrap();
}
