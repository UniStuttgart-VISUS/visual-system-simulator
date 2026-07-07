#![allow(non_snake_case)]
#![cfg(target_os = "android")]

use std::ffi::{c_void, CString};
use std::io::{Cursor, Read};
use std::panic;
use std::ptr::NonNull;
use std::sync::mpsc::{self, Receiver, SyncSender};
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

unsafe impl HasWindowHandle for AndroidHandle {
    fn raw_window_handle(&self) -> RawWindowHandle {
        self.0
    }
}

unsafe impl HasDisplayHandle for AndroidHandle {
    fn raw_display_handle(&self) -> RawDisplayHandle {
        RawDisplayHandle::Android(AndroidDisplayHandle::empty())
    }
}

struct CameraStream {
    upload: UploadYuvBuffer,
    frame_receiver: Receiver<YuvBuffer>,
}

impl CameraStream {
    fn new(context: &RenderContext, frame_receiver: Receiver<YuvBuffer>) -> Self {
        let mut upload = UploadYuvBuffer::new(context);
        upload.set_format(YuvFormat::_420888);
        CameraStream {
            upload,
            frame_receiver,
        }
    }
}

impl Node for CameraStream {
    fn name(&self) -> &'static str {
        "CameraStream"
    }

    fn negociate_slots(
        &mut self,
        context: &RenderContext,
        slots: NodeSlots,
        original_image: &mut Option<Texture>,
    ) -> NodeSlots {
        Node::negociate_slots(&mut self.upload, context, slots, original_image)
    }

    fn input(&mut self, eye: &EyeInput, mouse: &MouseInput) -> (EyeInput, NodeChanges) {
        // Uploading the buffer here is a bit sketchy but works.
        let mut changes = NodeChanges::empty();
        if let Ok(buffer) = self.frame_receiver.try_recv() {
            debug!("Uploading {}x{}px frame...", buffer.width, buffer.height);
            self.upload.upload_buffer(buffer);
            changes |= NodeChanges::OUTPUT;
        }
        let (eye, input_changes) = Node::input(&mut self.upload, eye, mouse);
        (eye, (changes | input_changes).normalized())
    }

    fn render(
        &mut self,
        context: &RenderContext,
        encoder: &mut wgpu::CommandEncoder,
        screen: Option<&RenderTexture>,
    ) {
        Node::render(&mut self.upload, context, encoder, screen);
    }

    fn post_render(&mut self, context: &RenderContext) {
        Node::post_render(&mut self.upload, context);
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
            .with_tag("libvss-mobile"),
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
        )
    };

    set_load(Box::new(move |full_path| {
        let full_path = CString::new(full_path)
            .map_err(|err| format!("Cannot open asset path '{}': {err}", full_path))?;
        let mut asset = assetManager
            .open(&full_path)
            .map_err(|_| format!("Cannot open asset '{}'", full_path.to_string_lossy()))?;
        let mut buffer = Vec::new();
        asset
            .read_to_end(&mut buffer)
            .map_err(|err| format!("Cannot read asset '{}': {err}", full_path.to_string_lossy()))?;
        Ok(Cursor::new(buffer))
    }));

    //TODO for testing purposes only
    // value_map.insert("peacock_cb_onoff".into(), Value::Bool(true));
    // value_map.insert("peacock_cb_strength".into(), Value::Number(1.0 as f64));
    // value_map.insert("peacock_cb_type".into(), Value::Number(0.0 as f64));
    // value_map.insert("colorblindness_onoff".into(), Value::Bool(true));
    // value_map.insert("colorblindness_type".into(), Value::Number(0.0 as f64));
    // value_map.insert("colorblindness_int".into(), Value::Number(100.0 as f64));
    // value_map.insert("cubemap_scale".into(), Value::Number(0.1 as f64));

    let mut window_handle = AndroidNdkWindowHandle::empty();
    window_handle.a_native_window = window.ptr().as_ptr() as *mut c_void;
    let handle = AndroidHandle(RawWindowHandle::AndroidNdk(window_handle));
    let size = [window.width() as u32, window.height() as u32];
    let surface = vss::Surface::new(size, handle, 1);
    let mut surface = futures::executor::block_on(surface);

    let (tx, rx) = mpsc::sync_channel(2);
    build_flow(&mut surface, rx);

    *guard = Some(Bridge {
        surface,
        frame_sender: tx,
        current_size: [1, 1],
        new_size: [1, 1],
    });
}

fn build_flow(surface: &mut Surface, frame_receiver: Receiver<YuvBuffer>) {
    //TODO: use a proper set of nodes.

    // Camera node.
    let node = CameraStream::new(surface, frame_receiver);
    surface.add_node(Box::new(node), 0);

    // Visual system passes.
    let node = Cataract::new(surface);
    surface.add_node(Box::new(node), 0);
    // let node = Lens::new(surface);
    // surface.add_node(Box::new(node), 0);
    let node = Retina::new(surface);
    surface.add_node(Box::new(node), 0);
    let node = PeacockCB::new(surface);
    surface.add_node(Box::new(node), 0);

    // Display node.
    let mut node = Display::new(surface);
    node.set_output_scale(OutputScale::Fill);
    surface.add_node(Box::new(node), 0);

    surface.negociate_slots();
}

#[no_mangle]
pub extern "system" fn Java_com_vss_simulator_SimulatorBridge_nativeDestroy(
    _env: JNIEnv<'_>,
    _class: JClass,
) {
    let mut guard: MutexGuard<'_, Option<Bridge>> = BRIDGE.lock().unwrap();
    *guard = None;
}

#[no_mangle]
pub extern "system" fn Java_com_vss_simulator_SimulatorBridge_nativeResize(
    _env: JNIEnv<'_>,
    _class: JClass,
    width: jint,
    height: jint,
) {
    let mut guard: MutexGuard<'_, Option<Bridge>> = BRIDGE.lock().unwrap();
    let bridge = (*guard).as_mut().expect("Bridge should be created");
    bridge.surface.resize([width as u32, height as u32]);
}

#[no_mangle]
pub extern "system" fn Java_com_vss_simulator_SimulatorBridge_nativeDraw(
    _env: JNIEnv<'_>,
    _class: JClass,
) {
    let mut guard: MutexGuard<'_, Option<Bridge>> = BRIDGE.lock().unwrap();
    let bridge = (*guard).as_mut().expect("Bridge should be created");
    // Fake input event for uploading and perspetive computation.
    for flow in bridge.surface.flows.iter() {
        let changes = flow.input(&MouseInput::default());
        bridge.surface.apply_changes(changes);
    }
    if (bridge.new_size[0] != bridge.current_size[0])
        || (bridge.new_size[1] != bridge.current_size[1])
    {
        debug!(
            "Buffer sizes do not match, old({}, {}), new({}, {})",
            bridge.current_size[0], bridge.current_size[1], bridge.new_size[0], bridge.new_size[1]
        );
        bridge.current_size = bridge.new_size;
        bridge.surface.negociate_slots();
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

    let pixels_y = env.convert_byte_array(y).unwrap().into_boxed_slice();
    let pixels_u = env.convert_byte_array(u).unwrap().into_boxed_slice();
    let pixels_v = env.convert_byte_array(v).unwrap().into_boxed_slice();

    let buffer = YuvBuffer {
        pixels_y,
        pixels_u,
        pixels_v,
        width: width as u32,
        height: height as u32,
    };

    let res = bridge.frame_sender.try_send(buffer);
    if res.is_ok() {
        bridge.new_size = [width, height];
    } else {
        warn!("{}, dropping frame", res.err().unwrap());
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

    let inspector = FromJsonInspector::try_new(&json_string);
    match inspector {
        Ok(mut inspector) => {
            let result = bridge.surface.inspect(&mut inspector);
            if result.contains(NodeChanges::SLOTS) {
                bridge.surface.negociate_slots();
            }
        }
        Err(err) => {
            error!("{:?}", err);
        }
    }
}

#[no_mangle]
pub extern "system" fn Java_com_vss_simulator_SimulatorBridge_nativeQuerySettings<'local>(
    env: JNIEnv<'local>,
    _class: JClass,
) -> JString<'local> {
    let mut guard: MutexGuard<'_, Option<Bridge>> = BRIDGE.lock().unwrap();
    let bridge = (*guard).as_mut().expect("Bridge should be created");

    let mut inspector = ToJsonInspector::new();
    bridge.surface.inspect(&mut inspector);
    let json_string = inspector.to_string();

    return env.new_string(json_string).unwrap();
}
