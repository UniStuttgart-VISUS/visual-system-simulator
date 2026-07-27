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

use crate::frame::{CameraFrame, FrameNode};

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
}
unsafe impl Send for Bridge {}

static BRIDGE: Mutex<Option<Bridge>> = Mutex::new(None);
static PENDING_FRAME: Mutex<Option<CameraFrame>> = Mutex::new(None);
static PENDING_SETTINGS: Mutex<Option<String>> = Mutex::new(None);

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
    let mut surface = Surface::new(size, handle, 1);
    surface.set_asset_loader(move |id: &AssetId| {
        let path = std::path::Path::new(&root).join(id.raw());
        std::fs::read(&path)
            .map(Cursor::new)
            .map_err(|e| format!("Cannot read {}: {e}", path.display()))
    });
    let node = FrameNode::new(&surface, &PENDING_FRAME);
    surface.add_node(Box::new(node), 0);
    let node = Cataract::new(&surface);
    surface.add_node(Box::new(node), 0);
    let node = EyeControl::new(&surface);
    surface.add_node(Box::new(node), 0);
    let node = Lens::new(&surface);
    surface.add_node(Box::new(node), 0);
    let node = Retina::new(&surface);
    surface.add_node(Box::new(node), 0);
    let node = PeacockCB::new(&surface);
    surface.add_node(Box::new(node), 0);
    let mut display = Display::new(&surface);
    display.set_output_scale(OutputScale::Fill);
    surface.add_node(Box::new(display), 0);
    surface.negociate_slots();
    *BRIDGE.lock().unwrap() = Some(Bridge {
        surface,
        frame_size: [1, 1],
    });
    true
}

#[no_mangle]
pub extern "C" fn vss_destroy() {
    *PENDING_FRAME.lock().unwrap() = None;
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
        *PENDING_FRAME.lock().unwrap() = Some(frame);
    }
}

#[no_mangle]
pub extern "C" fn vss_draw() {
    let mut guard = BRIDGE.lock().unwrap();
    let Some(b) = guard.as_mut() else { return };
    // Drop the frame lock before slot negotiation. FrameNode reads the same
    // mutex while selecting the camera render-target dimensions.
    let pending_size = {
        PENDING_FRAME
            .lock()
            .unwrap()
            .as_ref()
            .map(CameraFrame::output_size)
    };
    if let Some(size) = pending_size {
        if size != b.frame_size {
            b.frame_size = size;
            b.surface.negociate_slots();
        }
    }
    if let Some(json) = PENDING_SETTINGS.lock().unwrap().take() {
        apply_settings(b, &json);
    }
    for flow in &b.surface.flows {
        let changes = flow.input(&MouseInput::default());
        b.surface.apply_changes(changes);
    }
    b.surface.draw();
}

#[no_mangle]
pub unsafe extern "C" fn vss_post_settings(json: *const c_char) -> bool {
    if json.is_null() {
        return false;
    }
    let Ok(text) = CStr::from_ptr(json).to_str() else {
        return false;
    };
    if !matches!(
        serde_json::from_str::<serde_json::Value>(text),
        Ok(serde_json::Value::Object(_))
    ) {
        return false;
    }
    *PENDING_SETTINGS.lock().unwrap() = Some(text.to_owned());
    true
}

fn apply_settings(bridge: &mut Bridge, json: &str) {
    let Ok(values) = serde_json::from_str::<serde_json::Map<String, serde_json::Value>>(json)
    else {
        return;
    };
    let (doc, diagnostics) =
        vss_catalog::parse_config_value("ios", &serde_json::json!({"both": values}));
    if !diagnostics.is_empty() {
        return;
    }
    let settings = doc.effective_left().value_map();
    if vss_catalog::validate_settings(&settings).is_err() {
        return;
    }
    let patches: Result<Vec<_>, _> = bridge
        .surface
        .flows
        .iter()
        .map(|flow| {
            vss_catalog::compile_parameter_patch(flow, &settings, &|_, r| AssetId::from_str(r))
        })
        .collect();
    let Ok(patches) = patches else { return };
    let changes = bridge
        .surface
        .flows
        .iter()
        .zip(patches)
        .fold(NodeChanges::empty(), |c, (f, p)| c | p.apply(f));
    if changes.contains(NodeChanges::SLOTS) {
        bridge.surface.negociate_slots();
    }
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
