#![allow(non_snake_case)]
#![cfg(target_os = "android")]

use std::ffi::{c_void, CString};
use std::io::{Cursor, Read};
use std::panic;
use std::ptr::NonNull;
use std::sync::{Arc, Mutex, MutexGuard};

use log::*;

use jni::objects::{JByteBuffer, JClass, JObject, JString};
use jni::sys::{jfloat, jint};
use jni::{EnvUnowned, Outcome};

use ndk_sys;

use android_logger::Config;

use raw_window_handle::*;

use vss::*;

use crate::node::frame::{oriented_size, Frame, FrameNode, HardwareBufferFrame, SharedFrame};
use vss_catalog::Locale;

struct AndroidHandle(RawWindowHandle);

unsafe impl Send for AndroidHandle {}
unsafe impl Sync for AndroidHandle {}

impl HasWindowHandle for AndroidHandle {
    fn window_handle(&self) -> Result<WindowHandle<'_>, HandleError> {
        Ok(unsafe { WindowHandle::borrow_raw(self.0) })
    }
}

impl HasDisplayHandle for AndroidHandle {
    fn display_handle(&self) -> Result<DisplayHandle<'_>, HandleError> {
        Ok(unsafe {
            DisplayHandle::borrow_raw(RawDisplayHandle::Android(AndroidDisplayHandle::new()))
        })
    }
}

struct Bridge {
    pub surface: Surface<'static>,
    pub current_layout: [i32; 3],
    pub new_layout: [i32; 3],
    pub pose: IntentionalPose,
}

unsafe impl Send for Bridge {}

lazy_static::lazy_static! {
    static ref BRIDGE : Mutex<Option<Bridge>> = Mutex::new(None);
    static ref SHARED_FRAME: Arc<Mutex<SharedFrame>> = Arc::new(Mutex::new(SharedFrame::default()));
    static ref PENDING_FRAME_LAYOUT: Mutex<Option<[i32; 3]>> = Mutex::new(None);
    static ref PENDING_SETTINGS: Mutex<Option<(String, String)>> = Mutex::new(None);
}

#[no_mangle]
pub extern "system" fn Java_com_vss_simulator_SimulatorBridge_nativeCreate<'local>(
    mut env: EnvUnowned<'local>,
    _class: JClass,
    surface: JObject<'local>,
    assetManager: JObject<'local>,
) {
    android_logger::init_once(
        Config::default()
            .with_max_level(LevelFilter::Trace)
            .with_tag("libvss-android"),
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

    let (raw_env, raw_surface, raw_asset_manager) = match env
        .with_env_no_catch(|env| -> jni::errors::Result<_> {
            Ok((
                env.get_raw() as *mut c_void,
                surface.as_raw() as *mut c_void,
                assetManager.as_raw() as *mut c_void,
            ))
        })
        .into_outcome()
    {
        Outcome::Ok(raw) => raw,
        Outcome::Err(_) | Outcome::Panic(_) => panic!("JNI environment should be valid"),
    };

    let window = unsafe {
        ndk::native_window::NativeWindow::from_ptr(
            NonNull::new(ndk_sys::ANativeWindow_fromSurface(
                raw_env as *mut _,
                raw_surface as *mut _,
            ))
            .unwrap(),
        )
    };
    let assetManager = unsafe {
        ndk::asset::AssetManager::from_ptr(
            NonNull::new(ndk_sys::AAssetManager_fromJava(
                raw_env as *mut _,
                raw_asset_manager as *mut _,
            ))
            .unwrap(),
        )
    };

    let asset_loader = move |id: &AssetId| {
        let full_path = CString::new(id.raw())
            .map_err(|err| format!("Cannot open asset path '{}': {err}", id))?;
        let mut asset = assetManager
            .open(&full_path)
            .ok_or_else(|| format!("Cannot open asset '{}'", full_path.to_string_lossy()))?;
        let mut buffer = Vec::new();
        asset
            .read_to_end(&mut buffer)
            .map_err(|err| format!("Cannot read asset '{}': {err}", full_path.to_string_lossy()))?;
        Ok(Cursor::new(buffer))
    };

    //TODO for testing purposes only
    // value_map.insert("peacock_cb_onoff".into(), Value::Bool(true));
    // value_map.insert("peacock_cb_strength".into(), Value::Number(1.0 as f64));
    // value_map.insert("peacock_cb_type".into(), Value::Number(0.0 as f64));
    // value_map.insert("colorblindness_onoff".into(), Value::Bool(true));
    // value_map.insert("colorblindness_type".into(), Value::Number(0.0 as f64));
    // value_map.insert("colorblindness_int".into(), Value::Number(100.0 as f64));
    // value_map.insert("cubemap_scale".into(), Value::Number(0.1 as f64));

    let window_handle =
        AndroidNdkWindowHandle::new(NonNull::new(window.ptr().as_ptr() as *mut c_void).unwrap());
    let handle = AndroidHandle(RawWindowHandle::AndroidNdk(window_handle));
    let size = [window.width() as u32, window.height() as u32];
    let mut surface = vss::Surface::new(size, handle, 2);
    surface.set_asset_loader(asset_loader);

    build_flow(&mut surface, 0, SHARED_FRAME.clone());
    build_flow(&mut surface, 1, SHARED_FRAME.clone());
    surface.negociate_slots();
    surface.set_eye_mode(EyeMode::Left);

    *guard = Some(Bridge {
        surface,
        current_layout: [1, 1, 0],
        new_layout: [1, 1, 0],
        pose: IntentionalPose::default(),
    });
}

fn build_flow(surface: &mut Surface, flow_index: usize, shared_frame: Arc<Mutex<SharedFrame>>) {
    let node = FrameNode::new(surface, shared_frame);
    surface.add_node(Box::new(node), flow_index);

    // Visual system passes.
    let node = Cataract::new(surface);
    surface.add_node(Box::new(node), flow_index);
    let node = EyeControl::new(surface);
    surface.add_node(Box::new(node), flow_index);
    let node = Lens::new(surface);
    surface.add_node(Box::new(node), flow_index);
    let node = Retina::new(surface);
    surface.add_node(Box::new(node), flow_index);
    let node = PeacockCB::new(surface);
    surface.add_node(Box::new(node), flow_index);

    // Display node.
    let mut node = Display::new(surface);
    node.set_output_scale(OutputScale::Fill);
    surface.add_node(Box::new(node), flow_index);
}

#[no_mangle]
pub extern "system" fn Java_com_vss_simulator_SimulatorBridge_nativePostHardwareBuffer<'local>(
    mut env: EnvUnowned<'local>,
    _class: JClass,
    width: jint,
    height: jint,
    data_space: jint,
    rotation_degrees: jint,
    hardware_buffer: JObject<'local>,
) {
    let Some(frame) = HardwareBufferFrame::from_jni(
        &mut env,
        hardware_buffer,
        width as u32,
        height as u32,
        data_space,
        rotation_degrees,
    ) else {
        warn!("Received null HardwareBuffer, dropping frame");
        return;
    };

    SHARED_FRAME.lock().unwrap().publish(Frame::Hardware(frame));
    let size = oriented_size(width as u32, height as u32, rotation_degrees);
    *PENDING_FRAME_LAYOUT.lock().unwrap() = Some([
        size[0] as i32,
        size[1] as i32,
        rotation_degrees.rem_euclid(360),
    ]);
}

#[no_mangle]
pub extern "system" fn Java_com_vss_simulator_SimulatorBridge_nativePostRgba<'local>(
    mut env: EnvUnowned<'local>,
    _class: JClass,
    width: jint,
    height: jint,
    pixels: JByteBuffer<'local>,
) {
    let expected_len = (width.max(0) as usize)
        .saturating_mul(height.max(0) as usize)
        .saturating_mul(4);
    let data = env
        .with_env_no_catch(|env| -> jni::errors::Result<Vec<u8>> {
            let address = env.get_direct_buffer_address(&pixels)?;
            let capacity = env.get_direct_buffer_capacity(&pixels)?;
            Ok(unsafe { std::slice::from_raw_parts(address, capacity.min(expected_len)).to_vec() })
        })
        .into_outcome();
    let Outcome::Ok(data) = data else {
        warn!("Cannot read direct RGBA buffer");
        return;
    };
    if data.len() != expected_len {
        warn!(
            "Unexpected RGBA buffer length {}, expected {}",
            data.len(),
            expected_len
        );
        return;
    }

    let frame = Frame::Rgba(RgbBuffer {
        pixels_rgb: data.into_boxed_slice(),
        width: width as u32,
        height: height as u32,
    });
    SHARED_FRAME.lock().unwrap().publish(frame);
    *PENDING_FRAME_LAYOUT.lock().unwrap() = Some([width, height, 0]);
}

#[no_mangle]
pub extern "system" fn Java_com_vss_simulator_SimulatorBridge_nativeDestroy(
    _env: EnvUnowned<'_>,
    _class: JClass,
) {
    let mut guard: MutexGuard<'_, Option<Bridge>> = BRIDGE.lock().unwrap();
    *SHARED_FRAME.lock().unwrap() = SharedFrame::default();
    *PENDING_FRAME_LAYOUT.lock().unwrap() = None;
    *PENDING_SETTINGS.lock().unwrap() = None;
    *guard = None;
}

#[no_mangle]
pub extern "system" fn Java_com_vss_simulator_SimulatorBridge_nativeResize(
    _env: EnvUnowned<'_>,
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
    _env: EnvUnowned<'_>,
    _class: JClass,
) {
    let mut guard: MutexGuard<'_, Option<Bridge>> = BRIDGE.lock().unwrap();
    let bridge = (*guard).as_mut().expect("Bridge should be created");
    if let Some(layout) = PENDING_FRAME_LAYOUT.lock().unwrap().take() {
        bridge.new_layout = layout;
    }
    if let Some((left, right)) = PENDING_SETTINGS.lock().unwrap().take() {
        apply_settings(bridge, &left, &right);
    }
    // Fake input event for uploading and perspetive computation.
    for flow in bridge.surface.flows.iter() {
        let changes = flow.input(&MouseInput::default());
        bridge.surface.apply_changes(changes);
    }
    if bridge.new_layout != bridge.current_layout {
        debug!(
            "Frame layout changed, old={:?}, new={:?}",
            bridge.current_layout, bridge.new_layout
        );
        bridge.current_layout = bridge.new_layout;
        bridge.surface.negociate_slots();
    }
    bridge.surface.draw();
}

#[no_mangle]
pub extern "system" fn Java_com_vss_simulator_SimulatorBridge_nativePostSettings<'local>(
    mut env: EnvUnowned<'local>,
    _class: JClass,
    left_json: JString<'local>,
    right_json: JString<'local>,
) {
    let (left_json, right_json): (String, String) = match env
        .with_env_no_catch(|env| -> jni::errors::Result<_> {
            Ok((
                left_json.try_to_string(env)?,
                right_json.try_to_string(env)?,
            ))
        })
        .into_outcome()
    {
        Outcome::Ok(json) => json,
        Outcome::Err(err) => panic!("{}", err),
        Outcome::Panic(payload) => panic::resume_unwind(payload),
    };

    if compile_settings_json(&left_json).is_err() || compile_settings_json(&right_json).is_err() {
        error!("Settings for both eyes must be valid JSON objects");
        return;
    }
    *PENDING_SETTINGS.lock().unwrap() = Some((left_json, right_json));
}

#[no_mangle]
pub extern "system" fn Java_com_vss_simulator_SimulatorBridge_nativeSetEyeMode<'local>(
    mut env: EnvUnowned<'local>,
    _class: JClass,
    eye_mode: JString<'local>,
) {
    let eye_mode = match env
        .with_env_no_catch(|env| -> jni::errors::Result<String> { eye_mode.try_to_string(env) })
        .into_outcome()
    {
        Outcome::Ok(value) => value,
        Outcome::Err(err) => panic!("{}", err),
        Outcome::Panic(payload) => panic::resume_unwind(payload),
    };
    let mode = match eye_mode.as_str() {
        "left" => EyeMode::Left,
        "both" => EyeMode::Both,
        "right" => EyeMode::Right,
        _ => {
            error!("Eye mode must be left, both, or right");
            return;
        }
    };
    let mut guard = BRIDGE.lock().unwrap();
    guard
        .as_mut()
        .expect("Bridge should be created")
        .surface
        .set_eye_mode(mode);
}

#[no_mangle]
pub extern "system" fn Java_com_vss_simulator_SimulatorBridge_nativeSemanticInput<'local>(
    mut env: EnvUnowned<'local>,
    _class: JClass,
    kind: JString<'local>,
    x: jfloat,
    y: jfloat,
) {
    let kind = match env
        .with_env_no_catch(|env| -> jni::errors::Result<String> { kind.try_to_string(env) })
        .into_outcome()
    {
        Outcome::Ok(value) => value,
        Outcome::Err(err) => panic!("{}", err),
        Outcome::Panic(payload) => panic::resume_unwind(payload),
    };
    let input = match kind.as_str() {
        "gaze_delta" => SemanticInput::GazeDelta([x, y]),
        "view_delta" => SemanticInput::ViewDelta([x, y]),
        "reset_pose" => SemanticInput::ResetPose,
        _ => {
            error!("Unknown semantic input");
            return;
        }
    };
    let mut guard = BRIDGE.lock().unwrap();
    let bridge = guard.as_mut().expect("Bridge should be created");
    bridge.pose.apply(input);
    bridge.pose.apply_to_flows(&bridge.surface.flows);
    bridge.surface.apply_changes(NodeChanges::OUTPUT);
}

fn compile_settings_json(
    json: &str,
) -> Result<serde_json::Map<String, serde_json::Value>, serde_json::Error> {
    serde_json::from_str(json)
}

fn compile_settings(flow: &Flow, json: &str) -> Result<ParameterPatch, String> {
    let values =
        compile_settings_json(json).map_err(|err| format!("Invalid settings JSON: {err}"))?;
    let root = serde_json::json!({ "both": values });
    let (document, diagnostics) = vss_catalog::parse_config_value("android", &root);
    if !diagnostics.is_empty() {
        return Err(vss_catalog::diagnostics_to_string(&diagnostics));
    }
    let settings = document.effective_left().value_map();
    vss_catalog::validate_settings(&settings)
        .map_err(|diagnostics| vss_catalog::diagnostics_to_string(&diagnostics))?;
    vss_catalog::compile_parameter_patch(flow, &settings, &|_, reference| {
        AssetId::from_str(reference)
    })
    .map_err(|diagnostics| vss_catalog::diagnostics_to_string(&diagnostics))
}

fn apply_settings(bridge: &mut Bridge, left: &str, right: &str) {
    let patches = compile_settings(&bridge.surface.flows[0], left).and_then(|left| {
        compile_settings(&bridge.surface.flows[1], right).map(|right| (left, right))
    });
    let (left, right) = match patches {
        Ok(patches) => patches,
        Err(err) => {
            error!("{err}");
            return;
        }
    };
    let changes = left.apply(&bridge.surface.flows[0]) | right.apply(&bridge.surface.flows[1]);
    if changes.contains(NodeChanges::SLOTS) {
        bridge.surface.negociate_slots();
    }
}

#[no_mangle]
pub extern "system" fn Java_com_vss_simulator_SimulatorBridge_nativeCatalog<'local>(
    mut env: EnvUnowned<'local>,
    _class: JClass,
    locale: JString<'local>,
) -> JString<'local> {
    let tag = match env
        .with_env_no_catch(|env| -> jni::errors::Result<String> { locale.try_to_string(env) })
        .into_outcome()
    {
        Outcome::Ok(tag) => tag,
        Outcome::Err(err) => panic!("{}", err),
        Outcome::Panic(payload) => panic::resume_unwind(payload),
    };
    let json = vss_catalog::contract_json(Locale::from_tag(&tag));
    match env
        .with_env_no_catch(|env| env.new_string(json))
        .into_outcome()
    {
        Outcome::Ok(value) => value,
        Outcome::Err(err) => panic!("{}", err),
        Outcome::Panic(payload) => panic::resume_unwind(payload),
    }
}

#[no_mangle]
pub extern "system" fn Java_com_vss_simulator_SimulatorBridge_nativeComposeSettings<'local>(
    mut env: EnvUnowned<'local>,
    _class: JClass,
    locale: JString<'local>,
    active_presets: JString<'local>,
    manual_overrides: JString<'local>,
) -> JString<'local> {
    let (locale, active_presets, manual_overrides) = match env
        .with_env_no_catch(|env| -> jni::errors::Result<(String, String, String)> {
            Ok((
                locale.try_to_string(env)?,
                active_presets.try_to_string(env)?,
                manual_overrides.try_to_string(env)?,
            ))
        })
        .into_outcome()
    {
        Outcome::Ok(values) => values,
        Outcome::Err(err) => panic!("{}", err),
        Outcome::Panic(payload) => panic::resume_unwind(payload),
    };
    let active_presets: Vec<String> = serde_json::from_str(&active_presets)
        .unwrap_or_else(|err| panic!("Invalid active preset list: {}", err));
    let manual_overrides: serde_json::Map<String, serde_json::Value> =
        serde_json::from_str(&manual_overrides)
            .unwrap_or_else(|err| panic!("Invalid manual overrides: {}", err));
    let effective = vss_catalog::compose(
        Locale::from_tag(&locale),
        &active_presets,
        &manual_overrides,
    );
    let json = serde_json::to_string(&effective).expect("effective settings serialize");
    match env
        .with_env_no_catch(|env| env.new_string(json))
        .into_outcome()
    {
        Outcome::Ok(value) => value,
        Outcome::Err(err) => panic!("{}", err),
        Outcome::Panic(payload) => panic::resume_unwind(payload),
    }
}
