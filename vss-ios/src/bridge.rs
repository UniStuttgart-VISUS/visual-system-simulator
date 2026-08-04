use std::ffi::{c_char, c_void, CStr};
use std::io::Cursor;
use std::ptr::NonNull;
use std::sync::Mutex;

use raw_window_handle::{
    DisplayHandle, HandleError, HasDisplayHandle, HasWindowHandle, RawDisplayHandle,
    RawWindowHandle, UiKitDisplayHandle, UiKitWindowHandle, WindowHandle,
};
use vss::*;
use vss_catalog::Locale;

use crate::frame::{CameraFrame, FrameNode, SharedFrame};

struct UIKitHandle(RawWindowHandle);
unsafe impl Send for UIKitHandle {}
unsafe impl Sync for UIKitHandle {}
impl HasWindowHandle for UIKitHandle {
    fn window_handle(&self) -> Result<WindowHandle<'_>, HandleError> {
        Ok(unsafe { WindowHandle::borrow_raw(self.0) })
    }
}
impl HasDisplayHandle for UIKitHandle {
    fn display_handle(&self) -> Result<DisplayHandle<'_>, HandleError> {
        Ok(
            unsafe {
                DisplayHandle::borrow_raw(RawDisplayHandle::UiKit(UiKitDisplayHandle::new()))
            },
        )
    }
}

struct Bridge {
    surface: Surface<'static>,
    frame_size: [u32; 2],
    pose: IntentionalPose,
}
unsafe impl Send for Bridge {}

static BRIDGE: Mutex<Option<Bridge>> = Mutex::new(None);
static SHARED_FRAME: Mutex<SharedFrame> = Mutex::new(SharedFrame {
    generation: 0,
    frame: None,
});
static PENDING_SETTINGS: Mutex<Option<(String, String)>> = Mutex::new(None);

#[no_mangle]
pub unsafe extern "C" fn vss_create(
    view: *mut c_void,
    width: u32,
    height: u32,
    assets: *const c_char,
) -> bool {
    if view.is_null() || assets.is_null() {
        return false;
    }
    let _ = oslog::OsLogger::new("com.vss.ios")
        .level_filter(log::LevelFilter::Info)
        .init();
    let root = CStr::from_ptr(assets).to_string_lossy().into_owned();
    let handle = UIKitHandle(RawWindowHandle::UiKit(UiKitWindowHandle::new(
        NonNull::new_unchecked(view),
    )));
    let size = [width.max(1), height.max(1)];
    let mut surface = Surface::new(size, handle, 2);
    surface.set_asset_loader(move |id: &AssetId| {
        let path = std::path::Path::new(&root).join(id.raw());
        std::fs::read(&path)
            .map(Cursor::new)
            .map_err(|e| format!("Cannot read {}: {e}", path.display()))
    });
    for flow_index in 0..2 {
        let node = FrameNode::new(&surface, &SHARED_FRAME);
        surface.add_node(Box::new(node), flow_index);
        let node = Cataract::new(&surface);
        surface.add_node(Box::new(node), flow_index);
        let node = EyeControl::new(&surface);
        surface.add_node(Box::new(node), flow_index);
        let node = Lens::new(&surface);
        surface.add_node(Box::new(node), flow_index);
        let node = Retina::new(&surface);
        surface.add_node(Box::new(node), flow_index);
        let node = PeacockCB::new(&surface);
        surface.add_node(Box::new(node), flow_index);
        let mut display = Display::new(&surface);
        display.set_output_scale(OutputScale::Fill);
        surface.add_node(Box::new(display), flow_index);
    }
    surface.negociate_slots();
    surface.set_eye_mode(EyeMode::Left);
    *BRIDGE.lock().unwrap() = Some(Bridge {
        surface,
        frame_size: [1, 1],
        pose: IntentionalPose::default(),
    });
    true
}

#[no_mangle]
pub extern "C" fn vss_destroy() {
    *SHARED_FRAME.lock().unwrap() = SharedFrame::default();
    *BRIDGE.lock().unwrap() = None;
}
#[no_mangle]
pub extern "C" fn vss_resize(width: u32, height: u32) {
    if let Some(b) = BRIDGE.lock().unwrap().as_mut() {
        b.surface.resize([width.max(1), height.max(1)]);
    }
}

/// `luma`, `chroma`, and optional `depth` are retained MTLTexture objects backed by capture CVPixelBuffers.
#[no_mangle]
pub unsafe extern "C" fn vss_post_camera_frame(
    luma: *mut c_void,
    chroma: *mut c_void,
    depth: *mut c_void,
    width: u32,
    height: u32,
    depth_width: u32,
    depth_height: u32,
    rotation: i32,
    full_range: bool,
) {
    if let Some(frame) = CameraFrame::new(
        luma,
        chroma,
        depth,
        width,
        height,
        depth_width,
        depth_height,
        rotation,
        full_range,
    ) {
        SHARED_FRAME.lock().unwrap().publish(frame);
    }
}

#[no_mangle]
pub extern "C" fn vss_draw() {
    let mut guard = BRIDGE.lock().unwrap();
    let Some(b) = guard.as_mut() else { return };
    // Drop the frame lock before slot negotiation. FrameNode reads the same
    // mutex while selecting the camera render-target dimensions.
    let pending_size = {
        SHARED_FRAME
            .lock()
            .unwrap()
            .frame
            .as_ref()
            .map(CameraFrame::output_size)
    };
    if let Some(size) = pending_size {
        if size != b.frame_size {
            b.frame_size = size;
            b.surface.negociate_slots();
        }
    }
    if let Some((left, right)) = PENDING_SETTINGS.lock().unwrap().take() {
        apply_settings(b, &left, &right);
    }
    for flow in &b.surface.flows {
        let changes = flow.input(&MouseInput::default());
        b.surface.apply_changes(changes);
    }
    b.surface.draw();
}

#[no_mangle]
pub unsafe extern "C" fn vss_post_settings(left: *const c_char, right: *const c_char) -> bool {
    if left.is_null() || right.is_null() {
        return false;
    }
    let (Ok(left), Ok(right)) = (
        CStr::from_ptr(left).to_str(),
        CStr::from_ptr(right).to_str(),
    ) else {
        return false;
    };
    if [left, right].iter().any(|text| {
        !matches!(
            serde_json::from_str::<serde_json::Value>(text),
            Ok(serde_json::Value::Object(_))
        )
    }) {
        return false;
    }
    *PENDING_SETTINGS.lock().unwrap() = Some((left.to_owned(), right.to_owned()));
    true
}

fn compile_settings(flow: &Flow, json: &str) -> Option<ParameterPatch> {
    let Ok(values) = serde_json::from_str::<serde_json::Map<String, serde_json::Value>>(json)
    else {
        return None;
    };
    let (doc, diagnostics) =
        vss_catalog::parse_config_value("ios", &serde_json::json!({"both": values}));
    if !diagnostics.is_empty() {
        return None;
    }
    let settings = doc.effective_left().value_map();
    if vss_catalog::validate_settings(&settings).is_err() {
        return None;
    }
    vss_catalog::compile_parameter_patch(flow, &settings, &|_, r| AssetId::from_str(r)).ok()
}

fn apply_settings(bridge: &mut Bridge, left: &str, right: &str) {
    let (Some(left), Some(right)) = (
        compile_settings(&bridge.surface.flows[0], left),
        compile_settings(&bridge.surface.flows[1], right),
    ) else {
        return;
    };
    let changes = left.apply(&bridge.surface.flows[0]) | right.apply(&bridge.surface.flows[1]);
    if changes.contains(NodeChanges::SLOTS) {
        bridge.surface.negociate_slots();
    }
}

#[no_mangle]
pub unsafe extern "C" fn vss_set_eye_mode(value: *const c_char) -> bool {
    if value.is_null() {
        return false;
    }
    let mode = match CStr::from_ptr(value).to_str() {
        Ok("left") => EyeMode::Left,
        Ok("both") => EyeMode::Both,
        Ok("right") => EyeMode::Right,
        _ => return false,
    };
    let mut bridge = BRIDGE.lock().unwrap();
    let Some(bridge) = bridge.as_mut() else {
        return false;
    };
    bridge.surface.set_eye_mode(mode);
    true
}

#[no_mangle]
pub unsafe extern "C" fn vss_semantic_input(kind: *const c_char, x: f32, y: f32) -> bool {
    if kind.is_null() {
        return false;
    }
    let input = match CStr::from_ptr(kind).to_str() {
        Ok("gaze_delta") => SemanticInput::GazeDelta([x, y]),
        Ok("view_delta") => SemanticInput::ViewDelta([x, y]),
        Ok("reset_pose") => SemanticInput::ResetPose,
        _ => return false,
    };
    let mut bridge = BRIDGE.lock().unwrap();
    let Some(bridge) = bridge.as_mut() else {
        return false;
    };
    bridge.pose.apply(input);
    bridge.pose.apply_to_flows(&bridge.surface.flows);
    bridge.surface.apply_changes(NodeChanges::OUTPUT);
    true
}

#[no_mangle]
pub unsafe extern "C" fn vss_catalog_json(locale: *const c_char) -> *mut c_char {
    let tag = if locale.is_null() {
        "en"
    } else {
        CStr::from_ptr(locale).to_str().unwrap_or("en")
    };
    std::ffi::CString::new(vss_catalog::contract_json(Locale::from_tag(tag)))
        .unwrap()
        .into_raw()
}

#[no_mangle]
pub unsafe extern "C" fn vss_compose_settings(
    locale: *const c_char,
    active_presets: *const c_char,
    manual_overrides: *const c_char,
) -> *mut c_char {
    if active_presets.is_null() || manual_overrides.is_null() {
        return std::ptr::null_mut();
    }
    let tag = if locale.is_null() {
        "en"
    } else {
        CStr::from_ptr(locale).to_str().unwrap_or("en")
    };
    let Some(active) = CStr::from_ptr(active_presets)
        .to_str()
        .ok()
        .and_then(|value| serde_json::from_str::<Vec<String>>(value).ok())
    else {
        return std::ptr::null_mut();
    };
    let Some(manual) = CStr::from_ptr(manual_overrides)
        .to_str()
        .ok()
        .and_then(|value| {
            serde_json::from_str::<serde_json::Map<String, serde_json::Value>>(value).ok()
        })
    else {
        return std::ptr::null_mut();
    };
    let effective = vss_catalog::compose(Locale::from_tag(tag), &active, &manual);
    let Ok(json) = serde_json::to_string(&effective) else {
        return std::ptr::null_mut();
    };
    std::ffi::CString::new(json).map_or(std::ptr::null_mut(), std::ffi::CString::into_raw)
}
#[no_mangle]
pub unsafe extern "C" fn vss_string_free(value: *mut c_char) {
    if !value.is_null() {
        drop(std::ffi::CString::from_raw(value));
    }
}
