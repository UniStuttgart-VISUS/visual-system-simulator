use egui::{ComboBox, Context};
use egui_wgpu::{Renderer, RendererOptions, ScreenDescriptor};
use serde_json::{Map, Value};
use std::sync::Arc;
use vss::Surface;
use vss_catalog::{catalog, compose, ConfigDocument, ConfigSection, Control, Diagnostic, Locale};
use vss_winit::{EventResponse, WindowOverlay};
use winit::{
    event::{ElementState, WindowEvent},
    keyboard::{Key, NamedKey},
    window::Window,
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
                if let Some(preset) = catalog(self.locale).presets.iter().find(|p| p.id == preset) {
                    result.extend(
                        preset
                            .values
                            .iter()
                            .map(|(key, value)| ((*key).to_owned(), value.clone())),
                    );
                }
            }
            result.extend(local.manual.clone());
        }
        result
    }

    fn reset(&mut self, target: Target) {
        *self.layer_mut(target) = Layer::default();
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
}

impl DesktopGui {
    pub(crate) fn new(document: &ConfigDocument) -> Self {
        let system_locale = sys_locale::get_locale().unwrap_or_else(|| "en".into());
        let locale = Locale::from_tag(&system_locale);
        rust_i18n::set_locale(match locale {
            Locale::De => "de",
            Locale::En => "en",
        });
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
            target: Target::Both,
            dirty: true,
            diagnostics: Vec::new(),
            visible: true,
        }
    }

    fn ui(&mut self, root_ui: &mut egui::Ui) {
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
                    ui.selectable_value(&mut self.target, Target::Both, t!("eyes.both"));
                    ui.selectable_value(&mut self.target, Target::Left, t!("eyes.left"));
                    ui.selectable_value(&mut self.target, Target::Right, t!("eyes.right"));
                });
                ui.style_mut().spacing.scroll = egui::style::ScrollStyle::solid();
                egui::ScrollArea::vertical().show(ui, |ui| {
                    egui::CollapsingHeader::new(t!("presets.title"))
                        .default_open(true)
                        .show(ui, |ui| {
                            let active = &mut self.layers.layer_mut(target).presets;
                            for preset in &catalog.presets {
                                let mut enabled = active.iter().any(|id| id == preset.id);
                                if ui.checkbox(&mut enabled, preset.label.as_str()).changed() {
                                    if enabled {
                                        active.push(preset.id.to_owned());
                                    } else {
                                        active.retain(|id| id != preset.id);
                                    }
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
                                            self.layers.layer_mut(target).manual.remove(setting.id);
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
                                            self.layers
                                                .layer_mut(target)
                                                .manual
                                                .insert(setting.id.to_owned(), value);
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
                && event.logical_key == Key::Named(NamedKey::Escape))
        {
            self.visible = !self.visible;
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
                self.ui(ui);
            }
        });
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
                    .values
                    .keys()
                    .find(|key| second.values.contains_key(**key))
                    .map(|key| (first, second, *key))
            })
        }) else {
            return;
        };
        state.both.presets = vec![first.id.into(), second.id.into()];
        assert_eq!(
            state.effective_values(Target::Both)[key],
            second.values[key]
        );
        state.both.manual.insert(key.into(), json!(42.0));
        assert_eq!(state.effective_values(Target::Both)[key], json!(42.0));
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
}
