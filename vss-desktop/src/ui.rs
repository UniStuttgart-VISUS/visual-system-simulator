use egui::{ComboBox, Context};
use egui_wgpu::{Renderer, RendererOptions, ScreenDescriptor};
use serde_json::{Map, Value};
use std::{
    collections::BTreeSet,
    sync::{Arc, RwLock},
};
use vss::{EyeMode, Surface};
use vss_catalog::{catalog, compose, ConfigDocument, ConfigSection, Control, Diagnostic, Locale};
use vss_winit::{EventResponse, WindowOverlay};
use winit::{
    event::{ElementState, WindowEvent},
    keyboard::{Key, NamedKey},
    window::{Fullscreen, Window},
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Target {
    Both,
    Left,
    Right,
}

#[derive(Clone, Debug, Default)]
struct Layer {
    presets: Vec<String>,
    manual: Map<String, Value>,
    masked: BTreeSet<String>,
}

#[derive(Clone, Debug)]
pub(crate) struct LayerState {
    locale: Locale,
    both: Layer,
    left: Layer,
    right: Layer,
}

impl LayerState {
    pub(crate) fn from_document(locale: Locale, document: &ConfigDocument) -> Self {
        let catalog = catalog(locale);
        let convert = |section: &ConfigSection| Layer {
            presets: Vec::new(),
            manual: catalog
                .groups
                .iter()
                .flat_map(|group| &group.settings)
                .filter_map(|setting| {
                    section
                        .values
                        .get(setting.id)
                        .map(|value| (setting.id.to_owned(), value.value.clone()))
                })
                .collect(),
            masked: BTreeSet::new(),
        };
        Self {
            locale,
            both: convert(&document.both),
            left: convert(&document.left),
            right: convert(&document.right),
        }
    }

    fn layer(&self, target: Target) -> &Layer {
        match target {
            Target::Both => &self.both,
            Target::Left => &self.left,
            Target::Right => &self.right,
        }
    }

    fn layer_mut(&mut self, target: Target) -> &mut Layer {
        match target {
            Target::Both => &mut self.both,
            Target::Left => &mut self.left,
            Target::Right => &mut self.right,
        }
    }

    fn local_values(&self, target: Target) -> Map<String, Value> {
        let layer = self.layer(target);
        compose(self.locale, &layer.presets, &layer.manual)
    }

    fn effective_values(&self, target: Target) -> Map<String, Value> {
        let mut result = self.local_values(Target::Both);
        if target != Target::Both {
            let local = self.layer(target);
            for preset in &local.presets {
                if let Some(preset) = catalog(self.locale)
                    .presets
                    .iter()
                    .find(|candidate| candidate.id == *preset)
                {
                    let values = if !preset.both.is_empty() {
                        &preset.both
                    } else {
                        match target {
                            Target::Left => &preset.left,
                            Target::Right => &preset.right,
                            Target::Both => unreachable!(),
                        }
                    };
                    result.extend(
                        values
                            .iter()
                            .filter(|(key, _)| !local.masked.contains(*key))
                            .map(|(key, value)| ((*key).to_owned(), value.clone())),
                    );
                }
            }
            result.extend(
                local
                    .manual
                    .iter()
                    .filter(|(key, _)| !local.masked.contains(*key))
                    .map(|(key, value)| (key.clone(), value.clone())),
            );
        }
        result
    }

    fn set_manual(&mut self, target: Target, setting_id: &str, value: Value) {
        self.layer_mut(target)
            .manual
            .insert(setting_id.to_owned(), value);
        match target {
            Target::Both => {
                self.left.masked.insert(setting_id.to_owned());
                self.right.masked.insert(setting_id.to_owned());
            }
            Target::Left | Target::Right => {
                self.layer_mut(target).masked.remove(setting_id);
            }
        }
    }

    fn reset_setting(&mut self, target: Target, setting_id: &str) {
        self.layer_mut(target).manual.remove(setting_id);
        if target == Target::Both {
            self.left.masked.remove(setting_id);
            self.right.masked.remove(setting_id);
        }
    }

    fn reset(&mut self, target: Target) {
        *self.layer_mut(target) = Layer::default();
    }

    fn preset_enabled(&self, target: Target, preset_id: &str) -> bool {
        let catalog = catalog(self.locale);
        let Some(preset) = catalog.presets.iter().find(|preset| preset.id == preset_id) else {
            return false;
        };
        if !preset.both.is_empty() {
            self.layer(target).presets.iter().any(|id| id == preset_id)
        } else {
            (preset.left.is_empty() || self.left.presets.iter().any(|id| id == preset_id))
                && (preset.right.is_empty() || self.right.presets.iter().any(|id| id == preset_id))
        }
    }

    fn set_preset_enabled(&mut self, target: Target, preset_id: &str, enabled: bool) -> Target {
        let catalog = catalog(self.locale);
        let Some(preset) = catalog.presets.iter().find(|preset| preset.id == preset_id) else {
            return target;
        };
        let authored_left = !preset.left.is_empty();
        let authored_right = !preset.right.is_empty();
        let targets: &[Target] = if !preset.both.is_empty() {
            std::slice::from_ref(&target)
        } else {
            match (authored_left, authored_right) {
                (true, true) => &[Target::Left, Target::Right],
                (true, false) => &[Target::Left],
                (false, true) => &[Target::Right],
                (false, false) => unreachable!("catalog rejects empty presets"),
            }
        };
        for &layer_target in targets {
            let presets = &mut self.layer_mut(layer_target).presets;
            if enabled {
                if !presets.iter().any(|id| id == preset_id) {
                    presets.push(preset_id.to_owned());
                }
            } else {
                presets.retain(|id| id != preset_id);
            }
        }
        if enabled && (authored_left || authored_right) {
            Target::Both
        } else {
            target
        }
    }
}

impl Target {
    fn eye_mode(self) -> EyeMode {
        match self {
            Self::Left => EyeMode::Left,
            Self::Both => EyeMode::Both,
            Self::Right => EyeMode::Right,
        }
    }
}

pub(crate) struct DesktopGui {
    context: Context,
    state: Option<egui_winit::State>,
    renderer: Option<Renderer>,
    output: Option<egui::FullOutput>,
    paint_jobs: Vec<egui::ClippedPrimitive>,
    screen: ScreenDescriptor,
    layers: LayerState,
    target: Target,
    dirty: bool,
    diagnostics: Vec<Diagnostic>,
    visible: bool,
    fullscreen: bool,
    pending_open: bool,
    media_error: Option<String>,
    pose_input_size: Arc<RwLock<Option<[u32; 2]>>>,
}

impl DesktopGui {
    pub(crate) fn new(
        document: &ConfigDocument,
        pose_input_size: Arc<RwLock<Option<[u32; 2]>>>,
    ) -> Self {
        let system_locale = sys_locale::get_locale().unwrap_or_else(|| "en".into());
        let locale = Locale::from_tag(&system_locale);
        rust_i18n::set_locale(match locale {
            Locale::De => "de",
            Locale::En => "en",
        });
        let target = if document.left.values.is_empty() && document.right.values.is_empty() {
            Target::Left
        } else {
            Target::Both
        };
        Self {
            context: Context::default(),
            state: None,
            renderer: None,
            output: None,
            paint_jobs: Vec::new(),
            screen: ScreenDescriptor {
                size_in_pixels: [1, 1],
                pixels_per_point: 1.0,
            },
            layers: LayerState::from_document(locale, document),
            target,
            dirty: true,
            diagnostics: Vec::new(),
            visible: true,
            fullscreen: false,
            pending_open: false,
            media_error: None,
            pose_input_size,
        }
    }

    fn ui(&mut self, root_ui: &mut egui::Ui, window: &Window) {
        let locale = self.layers.locale;
        let catalog = catalog(locale);
        let target = self.target;
        let effective = self.layers.effective_values(target);
        egui::Panel::right("desktop_catalog")
            .resizable(false)
            .exact_size(360.0)
            .show_inside(root_ui, |ui| {
                ui.add(egui::Label::new(t!("panel.hide_hint")).selectable(false));
                ui.horizontal(|ui| {
                    ui.selectable_value(&mut self.target, Target::Left, t!("eyes.left"));
                    ui.selectable_value(&mut self.target, Target::Both, t!("eyes.both"));
                    ui.selectable_value(&mut self.target, Target::Right, t!("eyes.right"));
                });
                ui.horizontal(|ui| {
                    if ui.button(t!("output.open_file")).clicked() {
                        self.pending_open = true;
                    }
                    if ui
                        .button(if self.fullscreen {
                            t!("output.exit_fullscreen")
                        } else {
                            t!("output.fullscreen")
                        })
                        .clicked()
                    {
                        self.fullscreen = !self.fullscreen;
                        window.set_fullscreen(
                            self.fullscreen.then_some(Fullscreen::Borderless(None)),
                        );
                        window.request_redraw();
                    }
                });
                if let Some(error) = &self.media_error {
                    ui.colored_label(
                        egui::Color32::YELLOW,
                        format!("{}: {error}", t!("output.media_error")),
                    );
                }
                ui.style_mut().spacing.scroll = egui::style::ScrollStyle::solid();
                egui::ScrollArea::vertical().show(ui, |ui| {
                    egui::CollapsingHeader::new(t!("presets.title"))
                        .default_open(true)
                        .show(ui, |ui| {
                            for preset in &catalog.presets {
                                let mut enabled = self.layers.preset_enabled(target, &preset.id);
                                if ui.checkbox(&mut enabled, preset.label.as_str()).changed() {
                                    self.target =
                                        self.layers.set_preset_enabled(target, &preset.id, enabled);
                                    self.dirty = true;
                                }
                            }
                            if ui.button(t!("presets.reset_layer")).clicked() {
                                self.layers.reset(target);
                                self.dirty = true;
                            }
                        });
                    for group in &catalog.groups {
                        egui::CollapsingHeader::new(group.title.as_str())
                            .default_open(true)
                            .show(ui, |ui| {
                                for setting in &group.settings {
                                    let inherited = target != Target::Both
                                        && !self
                                            .layers
                                            .layer(target)
                                            .manual
                                            .contains_key(setting.id);
                                    let overridden =
                                        self.layers.layer(target).manual.contains_key(setting.id);
                                    let mut value = effective
                                        .get(setting.id)
                                        .cloned()
                                        .unwrap_or_else(|| setting.default.clone());
                                    ui.horizontal(|ui| {
                                        ui.label(setting.label.as_str())
                                            .on_hover_text(setting.help.as_str());
                                        if overridden
                                            && ui
                                                .small_button("↺")
                                                .on_hover_text(t!("settings.reset_value"))
                                                .clicked()
                                        {
                                            self.layers.reset_setting(target, setting.id);
                                            self.dirty = true;
                                        }
                                        let changed = ui
                                            .with_layout(
                                                egui::Layout::right_to_left(egui::Align::Center),
                                                |ui| {
                                                    edit_value(
                                                        ui,
                                                        setting.id,
                                                        &setting.control,
                                                        &mut value,
                                                    )
                                                },
                                            )
                                            .inner;
                                        if changed {
                                            self.layers.set_manual(target, setting.id, value);
                                            self.dirty = true;
                                        }
                                        if inherited {
                                            ui.weak(t!("settings.inherited"));
                                        }
                                    });
                                }
                            });
                    }
                });
                if !self.diagnostics.is_empty() {
                    ui.separator();
                    ui.colored_label(egui::Color32::YELLOW, t!("diagnostics.warning"));
                    for diagnostic in &self.diagnostics {
                        ui.small(format!("{}: {}", diagnostic.path, diagnostic.message));
                    }
                }
            });
    }
}

fn edit_value(ui: &mut egui::Ui, id: &str, control: &Control, value: &mut Value) -> bool {
    match control {
        Control::Boolean => {
            let mut v = value.as_bool().unwrap_or(false);
            let changed = ui.checkbox(&mut v, "").changed();
            *value = Value::Bool(v);
            changed
        }
        Control::Number {
            integer,
            min,
            max,
            step,
        } => {
            let mut v = value.as_f64().unwrap_or_default();
            let mut drag = egui::DragValue::new(&mut v).speed(*step);
            if let (Some(min), Some(max)) = (min, max) {
                drag = drag.range(*min..=*max);
            }
            let changed = ui.add(drag).changed();
            *value = if *integer {
                Value::from(v.round() as i64)
            } else {
                Value::from(v)
            };
            changed
        }
        Control::Choice { choices } => {
            let mut v = value.as_i64().unwrap_or_default() as i32;
            let before = v;
            ComboBox::from_id_salt(id)
                .selected_text(
                    choices
                        .iter()
                        .find(|c| c.value == v)
                        .map(|c| c.label.as_str())
                        .unwrap_or("—"),
                )
                .show_ui(ui, |ui| {
                    for choice in choices {
                        ui.selectable_value(&mut v, choice.value, choice.label.as_str());
                    }
                });
            *value = Value::from(v);
            v != before
        }
        Control::Text => {
            let mut v = value.as_str().unwrap_or_default().to_owned();
            let changed = ui.text_edit_singleline(&mut v).changed();
            *value = Value::String(v);
            changed
        }
        Control::Point => {
            let values = value
                .as_array()
                .cloned()
                .unwrap_or_else(|| vec![Value::from(0.0), Value::from(0.0)]);
            let mut x = values.first().and_then(Value::as_f64).unwrap_or_default();
            let mut y = values.get(1).and_then(Value::as_f64).unwrap_or_default();
            let changed = ui.add(egui::DragValue::new(&mut x)).changed()
                | ui.add(egui::DragValue::new(&mut y)).changed();
            *value = Value::Array(vec![Value::from(x), Value::from(y)]);
            changed
        }
    }
}

impl WindowOverlay for DesktopGui {
    fn initialize(&mut self, window: &Arc<Window>, surface: &Surface) {
        #[cfg(target_os = "windows")]
        {
            use windows_sys::Win32::{
                System::LibraryLoader::GetModuleHandleW,
                UI::WindowsAndMessaging::{
                    LoadIconW, SendMessageW, ICON_BIG, ICON_SMALL, WM_SETICON,
                },
            };
            use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};

            let RawWindowHandle::Win32(handle) = window.window_handle().unwrap().as_raw() else {
                unreachable!()
            };
            unsafe {
                let icon = LoadIconW(GetModuleHandleW(std::ptr::null()), 1usize as *const u16);
                assert!(!icon.is_null(), "failed to load Windows icon resource");
                let hwnd = handle.hwnd.get() as *mut std::ffi::c_void;
                SendMessageW(hwnd, WM_SETICON, ICON_BIG as usize, icon as isize);
                SendMessageW(hwnd, WM_SETICON, ICON_SMALL as usize, icon as isize);
            }
        }

        let weak_window = Arc::downgrade(window);
        self.context.set_request_repaint_callback(move |_| {
            if let Some(window) = weak_window.upgrade() {
                window.request_redraw();
            }
        });
        self.state = Some(egui_winit::State::new(
            self.context.clone(),
            egui::ViewportId::ROOT,
            window,
            None,
            None,
            None,
        ));
        self.renderer = Some(Renderer::new(
            surface.device(),
            surface.output_format(),
            RendererOptions::default(),
        ));
    }

    fn window_event(&mut self, window: &Window, event: &WindowEvent) -> EventResponse {
        if matches!(event, WindowEvent::KeyboardInput { event, .. }
            if event.state == ElementState::Pressed
                && !event.repeat
                && event.logical_key == Key::Named(NamedKey::Tab))
        {
            self.visible = !self.visible;
            window.request_redraw();
            return EventResponse {
                consumed: true,
                repaint: true,
            };
        }
        if self.fullscreen
            && matches!(event, WindowEvent::KeyboardInput { event, .. }
                if event.state == ElementState::Pressed
                    && !event.repeat
                    && event.logical_key == Key::Named(NamedKey::Escape))
        {
            self.fullscreen = false;
            window.set_fullscreen(None);
            window.request_redraw();
            return EventResponse {
                consumed: true,
                repaint: true,
            };
        }
        let response = self.state.as_mut().unwrap().on_window_event(window, event);
        EventResponse {
            consumed: response.consumed,
            repaint: response.repaint,
        }
    }

    fn prepare(&mut self, window: &Window, surface: &Surface) {
        self.fullscreen = window.fullscreen().is_some();
        surface.set_eye_mode(self.target.eye_mode());
        if self.dirty {
            let left = self.layers.effective_values(Target::Left);
            let right = self.layers.effective_values(Target::Right);
            let (changes, diagnostics) = crate::flow::apply_desktop_values(surface, &left, &right);
            surface.apply_changes(changes);
            self.diagnostics = diagnostics;
            self.dirty = false;
        }
        let input = self.state.as_mut().unwrap().take_egui_input(window);
        let context = self.context.clone();
        let output = context.run_ui(input, |ui| {
            if self.visible {
                self.ui(ui, window);
            }
        });
        if std::mem::take(&mut self.pending_open) {
            if let Some(input) = crate::cmd::pick_input_file() {
                match crate::flow::create_runtime_inputs(surface, &input) {
                    Ok(inputs) => {
                        surface.replace_node(0, inputs.left, 0);
                        surface.replace_node(0, inputs.right, 1);
                        *self.pose_input_size.write().unwrap() = inputs.input_size;
                        surface.negociate_slots();
                        surface.apply_changes(vss::NodeChanges::OUTPUT);
                        self.media_error = None;
                    }
                    Err(error) => self.media_error = Some(error.to_string()),
                }
            }
        }
        self.state
            .as_mut()
            .unwrap()
            .handle_platform_output(window, output.platform_output.clone());
        self.screen = ScreenDescriptor {
            size_in_pixels: [surface.width(), surface.height()],
            pixels_per_point: window.scale_factor() as f32,
        };
        self.paint_jobs = self
            .context
            .tessellate(output.shapes.clone(), output.pixels_per_point);
        self.output = Some(output);
    }

    fn render(
        &mut self,
        surface: &Surface,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
    ) {
        let renderer = self.renderer.as_mut().unwrap();
        let output = self.output.take().unwrap();
        for (id, delta) in &output.textures_delta.set {
            renderer.update_texture(surface.device(), surface.queue(), *id, delta);
        }
        renderer.update_buffers(
            surface.device(),
            surface.queue(),
            encoder,
            &self.paint_jobs,
            &self.screen,
        );
        let mut pass = encoder
            .begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Desktop GUI"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: target,
                    resolve_target: None,
                    depth_slice: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            })
            .forget_lifetime();
        renderer.render(&mut pass, &self.paint_jobs, &self.screen);
        drop(pass);
        for id in &output.textures_delta.free {
            renderer.free_texture(id);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn state() -> LayerState {
        LayerState::from_document(Locale::En, &ConfigDocument::default())
    }

    #[test]
    fn both_is_inherited_and_local_override_can_be_removed() {
        let mut state = state();
        state
            .both
            .manual
            .insert("cataract.blur".into(), json!(25.0));
        assert_eq!(
            state.effective_values(Target::Left)["cataract.blur"],
            json!(25.0)
        );
        state
            .left
            .manual
            .insert("cataract.blur".into(), json!(75.0));
        assert_eq!(
            state.effective_values(Target::Left)["cataract.blur"],
            json!(75.0)
        );
        state.left.manual.remove("cataract.blur");
        assert_eq!(
            state.effective_values(Target::Left)["cataract.blur"],
            json!(25.0)
        );
    }

    #[test]
    fn later_presets_then_manual_values_win() {
        let mut state = state();
        let presets = catalog(Locale::En).presets;
        let Some((first, second, key)) = presets.iter().enumerate().find_map(|(i, first)| {
            presets.iter().skip(i + 1).find_map(|second| {
                first
                    .both
                    .keys()
                    .find(|key| second.both.contains_key(*key))
                    .map(|key| (first, second, key.clone()))
            })
        }) else {
            return;
        };
        state.both.presets = vec![first.id.clone(), second.id.clone()];
        let mut expected = Map::new();
        expected.insert(key.clone(), second.both[&key].clone());
        vss_catalog::normalize(Locale::En, &mut expected);
        assert_eq!(state.effective_values(Target::Both)[&key], expected[&key]);
        state.both.manual.insert(key.clone(), json!(42.0));
        assert_eq!(state.effective_values(Target::Both)[&key], json!(42.0));
    }

    #[test]
    fn reset_only_clears_selected_layer() {
        let mut state = state();
        state
            .both
            .manual
            .insert("cataract.blur".into(), json!(10.0));
        state
            .left
            .manual
            .insert("cataract.blur".into(), json!(20.0));
        state.reset(Target::Left);
        assert!(state.left.manual.is_empty());
        assert!(!state.both.manual.is_empty());
    }

    #[test]
    fn intrinsic_preset_selects_both_then_symmetric_preset_targets_the_visible_eye() {
        let mut state = state();
        let mode = state.set_preset_enabled(Target::Left, "strabismus-esotropia-mild", true);
        assert_eq!(mode, Target::Both);
        assert_eq!(
            state.effective_values(Target::Left)["eye.axis-y"],
            json!(0.05)
        );
        assert_eq!(
            state.effective_values(Target::Right)["eye.axis-y"],
            json!(-0.05)
        );

        let mode = state.set_preset_enabled(Target::Left, "cataract-mild", true);
        assert_eq!(mode, Target::Left);
        assert!(state.left.presets.contains(&"cataract-mild".to_owned()));
        assert!(!state.right.presets.contains(&"cataract-mild".to_owned()));
        assert_eq!(
            state.effective_values(Target::Right)["eye.axis-y"],
            json!(-0.05)
        );
    }

    #[test]
    fn both_manual_edit_masks_only_matching_eye_values_and_reset_reveals_them() {
        let mut state = state();
        state.set_manual(Target::Left, "cataract.blur", json!(20.0));
        state.set_manual(Target::Right, "cataract.blur", json!(30.0));
        state.set_manual(Target::Right, "eye.axis-y", json!(-0.05));

        state.set_manual(Target::Both, "cataract.blur", json!(10.0));
        assert_eq!(
            state.effective_values(Target::Left)["cataract.blur"],
            json!(10.0)
        );
        assert_eq!(
            state.effective_values(Target::Right)["cataract.blur"],
            json!(10.0)
        );
        assert_eq!(
            state.effective_values(Target::Right)["eye.axis-y"],
            json!(-0.05)
        );

        state.reset_setting(Target::Both, "cataract.blur");
        assert_eq!(
            state.effective_values(Target::Left)["cataract.blur"],
            json!(20.0)
        );
        assert_eq!(
            state.effective_values(Target::Right)["cataract.blur"],
            json!(30.0)
        );
    }
}
