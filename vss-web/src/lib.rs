#![cfg(target_arch = "wasm32")]
use std::{
    cell::RefCell,
    rc::Rc,
    sync::mpsc::{self, Receiver, SyncSender},
};
use vss::{RgbBuffer, UploadRgbBuffer, *};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

struct UploadStream {
    upload: UploadRgbBuffer,
    frame_receiver: Receiver<(RgbBuffer, RgbInputFlags)>,
    shared: Rc<RefCell<SharedFrame>>,
}

#[derive(Default)]
struct SharedFrame {
    generation: u64,
    frame: Option<(Rc<RgbBuffer>, u32)>,
}

impl UploadStream {
    fn new(
        context: &RenderContext,
        frame_receiver: Receiver<(RgbBuffer, RgbInputFlags)>,
        shared: Rc<RefCell<SharedFrame>>,
    ) -> Self {
        UploadStream {
            upload: UploadRgbBuffer::new(context),
            frame_receiver,
            shared,
        }
    }
}

impl Node for UploadStream {
    fn name(&self) -> &'static str {
        "UploadStream"
    }

    fn input(&mut self, eye: &EyeInput, mouse: &MouseInput) -> (EyeInput, NodeChanges) {
        //XXX: web stream frame polling here. might not be the right place in the long term.
        let mut changes = NodeChanges::empty();
        if let Ok((buffer, flags)) = self.frame_receiver.try_recv() {
            let buffer = Rc::new(buffer);
            let flag_bits = flags.bits();
            self.upload.set_flags(flags);
            self.upload.upload_buffer(&buffer);
            let mut shared = self.shared.borrow_mut();
            shared.generation += 1;
            shared.frame = Some((buffer, flag_bits));
            changes |= NodeChanges::SLOTS;
        }
        let (eye, input_changes) = Node::input(&mut self.upload, eye, mouse);
        (eye, (changes | input_changes).normalized())
    }

    fn negociate_slots(
        &mut self,
        context: &RenderContext,
        slots: NodeSlots,
        original_image: &mut Option<Texture>,
    ) -> NodeSlots {
        Node::negociate_slots(&mut self.upload, context, slots, original_image)
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

struct UploadSharedStream {
    upload: UploadRgbBuffer,
    shared: Rc<RefCell<SharedFrame>>,
    generation: u64,
}

impl UploadSharedStream {
    fn new(context: &RenderContext, shared: Rc<RefCell<SharedFrame>>) -> Self {
        Self {
            upload: UploadRgbBuffer::new(context),
            shared,
            generation: 0,
        }
    }

    fn synchronize(&mut self) -> bool {
        let shared = self.shared.borrow();
        if shared.generation == self.generation {
            return false;
        }
        let Some((buffer, flag_bits)) = &shared.frame else {
            return false;
        };
        self.upload
            .set_flags(RgbInputFlags::from_bits_retain(*flag_bits));
        self.upload.upload_buffer(buffer);
        self.generation = shared.generation;
        true
    }
}

impl Node for UploadSharedStream {
    fn name(&self) -> &'static str {
        "UploadSharedStream"
    }

    fn input(&mut self, eye: &EyeInput, mouse: &MouseInput) -> (EyeInput, NodeChanges) {
        let changed = self.synchronize();
        let (eye, changes) = Node::input(&mut self.upload, eye, mouse);
        (
            eye,
            (changes | NodeChanges::from_output_slots(changed, false)).normalized(),
        )
    }

    fn negociate_slots(
        &mut self,
        context: &RenderContext,
        slots: NodeSlots,
        original_image: &mut Option<Texture>,
    ) -> NodeSlots {
        self.synchronize();
        Node::negociate_slots(&mut self.upload, context, slots, original_image)
    }

    fn render(
        &mut self,
        context: &RenderContext,
        encoder: &mut wgpu::CommandEncoder,
        screen: Option<&RenderTexture>,
    ) {
        self.synchronize();
        Node::render(&mut self.upload, context, encoder, screen)
    }
}

fn build_flow(surface: &mut Surface, flow_index: usize, input: Box<dyn Node>) {
    surface.add_node(input, flow_index);

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

#[wasm_bindgen]
pub struct Simulator {
    frame_sender: SyncSender<(RgbBuffer, RgbInputFlags)>,
    surface: Surface<'static>,
    canvas: web_sys::HtmlCanvasElement,
    pose: IntentionalPose,
}

#[wasm_bindgen]
impl Simulator {
    #[wasm_bindgen]
    pub async fn create(parent_id: &str) -> Result<Simulator, JsError> {
        std::panic::set_hook(Box::new(console_error_panic_hook::hook));
        let _ = console_log::init();

        let (tx, rx) = mpsc::sync_channel(2);
        let window = web_sys::window().ok_or_else(|| JsError::new("Window is unavailable"))?;
        let document = window
            .document()
            .ok_or_else(|| JsError::new("Document is unavailable"))?;
        let parent = document
            .get_element_by_id(parent_id)
            .ok_or_else(|| JsError::new("Canvas parent is missing"))?;
        let element = document
            .create_element("canvas")
            .map_err(|_| JsError::new("Cannot create canvas"))?;
        let canvas: web_sys::HtmlCanvasElement = element
            .dyn_into()
            .map_err(|_| JsError::new("Created element is not a canvas"))?;
        let ratio = window.device_pixel_ratio();
        let width = ((parent.client_width() as f64 * ratio).round() as u32).max(1);
        let height = ((parent.client_height() as f64 * ratio).round() as u32).max(1);
        canvas.set_width(width);
        canvas.set_height(height);
        parent
            .append_child(&canvas)
            .map_err(|_| JsError::new("Cannot attach canvas"))?;

        let instance = wgpu::Instance::default();
        let gpu_surface = instance
            .create_surface(wgpu::SurfaceTarget::Canvas(canvas.clone()))
            .map_err(|err| JsError::new(&format!("Cannot create WebGPU surface: {err}")))?;
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&gpu_surface),
                force_fallback_adapter: false,
            })
            .await
            .map_err(|err| JsError::new(&format!("Cannot create WebGPU adapter: {err}")))?;
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: None,
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::downlevel_webgl2_defaults()
                    .using_resolution(adapter.limits()),
                experimental_features: wgpu::ExperimentalFeatures::disabled(),
                trace: wgpu::Trace::Off,
                memory_hints: wgpu::MemoryHints::Performance,
            })
            .await
            .map_err(|err| JsError::new(&format!("Cannot create WebGPU device: {err}")))?;
        let mut surface =
            Surface::with_existing([width, height], 2, gpu_surface, adapter, device, queue).await;
        let shared = Rc::new(RefCell::new(SharedFrame::default()));
        let left = UploadStream::new(&surface, rx, shared.clone());
        let right = UploadSharedStream::new(&surface, shared);
        build_flow(&mut surface, 0, Box::new(left));
        build_flow(&mut surface, 1, Box::new(right));
        surface.negociate_slots();
        surface.set_eye_mode(EyeMode::Left);
        Ok(Simulator {
            frame_sender: tx,
            surface,
            canvas,
            pose: IntentionalPose::default(),
        })
    }

    #[wasm_bindgen]
    pub fn post_frame(
        &mut self,
        pixels: Vec<u8>,
        width: usize,
        height: usize,
        rgbd: bool,
    ) -> Result<(), JsError> {
        let buffer = RgbBuffer {
            pixels_rgb: pixels.into_boxed_slice(),
            width: width as u32,
            height: height as u32,
        };
        let flags = if rgbd {
            // Browser canvas data keeps the RGB half above the depth half. The
            // upload shader's RGB-D split uses texture coordinates with the
            // opposite vertical origin, so flip before selecting each half.
            RgbInputFlags::RGBD_HORIZONTAL | RgbInputFlags::VERTICALLY_FLIPPED
        } else {
            RgbInputFlags::empty()
        };
        self.frame_sender
            .try_send((buffer, flags))
            .map_err(|err| JsError::new(&err.to_string()))?;
        self.render();
        Ok(())
    }

    pub fn post_settings(&mut self, left: &str, right: &str) -> Result<(), JsError> {
        let changes =
            apply_settings(&self.surface, left, right).map_err(|err| JsError::new(&err))?;
        self.surface.apply_changes(changes);
        self.render();
        Ok(())
    }

    pub fn set_eye_mode(&mut self, eye_mode: &str) -> Result<(), JsError> {
        let mode = match eye_mode {
            "left" => EyeMode::Left,
            "both" => EyeMode::Both,
            "right" => EyeMode::Right,
            _ => return Err(JsError::new("Eye mode must be left, both, or right")),
        };
        self.surface.set_eye_mode(mode);
        self.render();
        Ok(())
    }

    pub fn semantic_input(&mut self, kind: &str, x: f32, y: f32) -> Result<(), JsError> {
        let input = match kind {
            "gaze_delta" => SemanticInput::GazeDelta([x, y]),
            "view_delta" => SemanticInput::ViewDelta([x, y]),
            "reset_pose" => SemanticInput::ResetPose,
            _ => return Err(JsError::new("Unknown semantic input")),
        };
        self.pose.apply(input);
        self.pose.apply_to_flows(&self.surface.flows);
        self.surface.apply_changes(NodeChanges::OUTPUT);
        self.render();
        Ok(())
    }

    pub fn resize(&mut self) {
        self.render();
    }

    pub fn destroy(&mut self) {
        self.canvas.remove();
    }
}

impl Simulator {
    fn render(&mut self) {
        let ratio = web_sys::window()
            .map(|window| window.device_pixel_ratio())
            .unwrap_or(1.0);
        let width = ((self.canvas.client_width().max(1) as f64 * ratio).round() as u32).max(1);
        let height = ((self.canvas.client_height().max(1) as f64 * ratio).round() as u32).max(1);
        if width != self.surface.width() || height != self.surface.height() {
            self.canvas.set_width(width);
            self.canvas.set_height(height);
            self.surface.resize([width, height]);
        }
        let changes = self
            .surface
            .flows
            .iter()
            .fold(NodeChanges::empty(), |changes, flow| {
                changes | flow.input(&MouseInput::default())
            });
        self.surface.apply_changes(changes);
        if self.surface.pending_changes().contains(NodeChanges::SLOTS) {
            self.surface.negociate_slots();
        }
        self.surface.draw();
    }
}

fn compile_settings(flow: &Flow, json: &str) -> Result<ParameterPatch, String> {
    let values = serde_json::from_str::<serde_json::Map<String, serde_json::Value>>(json)
        .map_err(|err| format!("Invalid settings JSON: {err}"))?;
    let root = serde_json::json!({ "both": values });
    let (document, diagnostics) = vss_catalog::parse_config_value("web", &root);
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

fn apply_settings(surface: &Surface, left: &str, right: &str) -> Result<NodeChanges, String> {
    let left = compile_settings(&surface.flows[0], left)?;
    let right = compile_settings(&surface.flows[1], right)?;
    Ok(left.apply(&surface.flows[0]) | right.apply(&surface.flows[1]))
}

#[wasm_bindgen]
pub fn catalog(locale: &str) -> String {
    vss_catalog::contract_json(vss_catalog::Locale::from_tag(locale))
}

#[wasm_bindgen]
pub fn compose_settings(
    locale: &str,
    active_presets: &str,
    manual_overrides: &str,
) -> Result<String, JsError> {
    let mut active: Vec<String> = serde_json::from_str(active_presets)
        .map_err(|err| JsError::new(&format!("Invalid preset list: {err}")))?;
    let manual = serde_json::from_str(manual_overrides)
        .map_err(|err| JsError::new(&format!("Invalid manual overrides: {err}")))?;
    let locale = vss_catalog::Locale::from_tag(locale);
    let catalog = vss_catalog::catalog(locale);
    active.sort_by_key(|id| {
        catalog
            .presets
            .iter()
            .position(|preset| &preset.id == id)
            .unwrap_or(usize::MAX)
    });
    serde_json::to_string(&vss_catalog::compose(locale, &active, &manual))
        .map_err(|err| JsError::new(&err.to_string()))
}
