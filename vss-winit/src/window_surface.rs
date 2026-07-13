use std::rc::Rc;
use std::sync::{Arc, RwLock};

use cgmath::{Matrix4, Rad, SquareMatrix, Vector3, Vector4};
use vss::*;
#[cfg(not(target_arch = "wasm32"))]
use winit::platform::run_on_demand::EventLoopExtRunOnDemand;
use winit::{
    application::ApplicationHandler, dpi::*, event::*, event_loop::ActiveEventLoop, window::Window,
};
#[cfg(not(target_arch = "wasm32"))]
use winit::{
    error::EventLoopError,
    event_loop::{ControlFlow, EventLoop},
};

#[derive(Clone, Copy, Debug, Default)]
pub struct EventResponse {
    pub consumed: bool,
    pub repaint: bool,
}

pub trait WindowOverlay {
    fn initialize(&mut self, window: &Arc<Window>, surface: &Surface);
    fn window_event(&mut self, window: &Window, event: &WindowEvent) -> EventResponse;
    fn prepare(&mut self, window: &Window, surface: &Surface);
    fn render(
        &mut self,
        surface: &Surface,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
    );
}

/// Represents a window along with its associated rendering context and [Flow].
pub struct WindowSurface {
    surface: Option<Rc<Surface<'static>>>,
    window: Option<Arc<Window>>,
    flow_count: usize,

    deferred_size: Option<PhysicalSize<u32>>,
    visible: bool,
    init_fn: Option<Box<dyn FnOnce(&mut Surface)>>,
    poll_fn: Box<dyn FnMut() -> bool>,
    update_fn: Option<Box<dyn FnMut(&Surface) -> NodeChanges>>,
    overlay: Option<Box<dyn WindowOverlay>>,

    active: bool,
    static_view: Option<(f32, f32)>,
    static_gaze: Option<(f32, f32)>,
    pose_input_size: Arc<RwLock<Option<[u32; 2]>>>,
    mouse: MouseInput,

    override_gaze: bool,
    override_view: bool,
    canvas_parent: Option<String>,
}

pub fn pose_from_position(
    position: (f32, f32),
    pose_size: [u32; 2],
) -> (Matrix4<f32>, Vector3<f32>) {
    let width = pose_size[0].max(1) as f32;
    let height = pose_size[1].max(1) as f32;
    let yaw = (position.0 / width - 0.5) * std::f32::consts::PI * 2.0;
    let pitch = (position.1 / height - 0.5) * std::f32::consts::PI;
    let view = Matrix4::from_angle_x(Rad(pitch)) * Matrix4::from_angle_y(Rad(yaw));
    let gaze = (view.invert().unwrap() * Vector4::unit_z()).truncate();
    (view, gaze)
}

impl WindowSurface {
    pub fn new<I, P>(
        visible: bool,
        flow_count: usize,
        static_view: Option<(f32, f32)>,
        static_gaze: Option<(f32, f32)>,
        pose_input_size: Arc<RwLock<Option<[u32; 2]>>>,
        init_fn: I,
        poll_fn: P,
    ) -> Self
    where
        I: 'static + FnOnce(&mut Surface),
        P: 'static + FnMut() -> bool,
    {
        Self {
            surface: None,
            window: None,
            flow_count,
            deferred_size: None,
            visible: visible,
            init_fn: Some(Box::new(init_fn)),
            poll_fn: Box::new(poll_fn),
            update_fn: None,
            overlay: None,
            active: false,
            static_view,
            static_gaze,
            pose_input_size,
            mouse: MouseInput {
                position: (0.0, 0.0),
                left_button: false,
                right_button: false,
            },
            override_view: false,
            override_gaze: false,
            canvas_parent: None,
        }
    }

    pub fn with_overlay(mut self, overlay: impl WindowOverlay + 'static) -> Self {
        self.overlay = Some(Box::new(overlay));
        self
    }

    pub fn with_canvas_parent(mut self, id: impl Into<String>) -> Self {
        self.canvas_parent = Some(id.into());
        self
    }

    pub fn with_update_fn(
        mut self,
        update_fn: impl 'static + FnMut(&Surface) -> NodeChanges,
    ) -> Self {
        self.update_fn = Some(Box::new(update_fn));
        self
    }

    #[cfg(target_arch = "wasm32")]
    pub fn spawn(self, event_loop: winit::event_loop::EventLoop<()>) {
        use winit::platform::web::EventLoopExtWebSys;
        event_loop.spawn_app(self);
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn run_app(mut self, event_loop: &mut EventLoop<()>) -> Result<(), EventLoopError> {
        event_loop.set_control_flow(ControlFlow::Wait);
        event_loop.run_app_on_demand(&mut self)
    }

    fn update_input(&self) {
        let surface = self.surface.clone().unwrap();
        let mut changes = NodeChanges::empty();
        for f in surface.flows.iter() {
            let pose_size = self
                .pose_input_size
                .read()
                .unwrap()
                .unwrap_or([surface.width(), surface.height()]);
            let view_position = self.static_view.or(if self.override_view {
                Some(self.mouse.position)
            } else {
                None
            });
            let gaze_position = self.static_gaze.or(if self.override_gaze {
                Some(self.mouse.position)
            } else {
                None
            });

            {
                let mut eye = f.eye_mut();

                if let Some(position) = view_position {
                    eye.view = pose_from_position(position, pose_size).0;
                }

                if let Some(position) = gaze_position {
                    eye.gaze = pose_from_position(position, pose_size).1;
                }
            }

            changes |= f.input(&self.mouse);
        }
        surface.apply_changes(changes);
    }

    fn update_size(&mut self, deferred_size: Option<PhysicalSize<u32>>) {
        let new_size = deferred_size;

        if let Some(new_size) = new_size {
            if let Some(surface) = &mut self.surface {
                let surface = Rc::get_mut(surface).unwrap();

                surface.resize([new_size.width, new_size.height]);
            }
        }
    }

    fn request_output(&self) {
        if let Some(surface) = &self.surface {
            surface.apply_changes(NodeChanges::OUTPUT);
        }
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }
}

impl ApplicationHandler for WindowSurface {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let window_attributes = Window::default_attributes()
            .with_title("Visual System Simulator")
            .with_min_inner_size(LogicalSize::new(640.0, 360.0))
            .with_inner_size(LogicalSize::new(1280.0, 720.0))
            .with_visible(self.visible);

        let window = Arc::new(event_loop.create_window(window_attributes).unwrap());
        #[cfg(target_arch = "wasm32")]
        if let Some(parent) = &self.canvas_parent {
            use winit::platform::web::WindowExtWebSys;
            let document = web_sys::window().unwrap().document().unwrap();
            document
                .get_element_by_id(parent)
                .expect("canvas parent is missing")
                .append_child(&web_sys::Element::from(
                    window.canvas().expect("window has no canvas"),
                ))
                .expect("cannot append canvas");
        }
        window.set_cursor_visible(true);
        let window_size = window.inner_size();

        let mut surface = Surface::new(
            [window_size.width, window_size.height],
            window.clone(),
            self.flow_count,
        );

        self.init_fn.take().expect("window initialized twice")(&mut surface);
        if let Some(overlay) = &mut self.overlay {
            overlay.initialize(&window, &surface);
        }
        if (self.poll_fn)() {
            self.window = Some(window);
            self.surface = Some(Rc::new(surface));
            event_loop.exit();
            return;
        }

        surface.negociate_slots();
        window.request_redraw();
        self.window = Some(window);
        self.surface = Some(Rc::new(surface));
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        let response = self
            .overlay
            .as_mut()
            .zip(self.window.as_ref())
            .map(|(overlay, window)| overlay.window_event(window, &event))
            .unwrap_or_default();
        if response.repaint {
            self.request_output();
        }

        match event {
            event if closes_window(&event) => {
                event_loop.exit();
            }
            WindowEvent::Focused(active) => {
                self.active = active;
            }
            WindowEvent::Resized(size) => {
                self.deferred_size = Some(size);
                self.request_output();
            }
            WindowEvent::CursorMoved { position, .. } => {
                if self.active && !response.consumed {
                    self.mouse.position = (position.x as f32, position.y as f32);
                    self.request_output();
                }
            }
            WindowEvent::CursorLeft { .. } => {
                if self.active {
                    self.override_view = false;
                    self.override_gaze = false;
                    //XXX: reset gaze?
                    self.request_output();
                }
            }
            WindowEvent::MouseInput { state, button, .. } => {
                if self.active
                    && !simulation_input_allowed(response.consumed)
                    && state == ElementState::Released
                {
                    match button {
                        MouseButton::Left => {
                            self.override_view = false;
                            self.mouse.left_button = false;
                        }
                        MouseButton::Right => {
                            self.override_gaze = false;
                            self.mouse.right_button = false;
                        }
                        _ => {}
                    }
                    self.request_output();
                } else if self.active && simulation_input_allowed(response.consumed) {
                    match button {
                        MouseButton::Left => {
                            self.override_view = state == ElementState::Pressed;
                            self.mouse.left_button = state == ElementState::Pressed;
                        }
                        MouseButton::Right => {
                            self.override_gaze = state == ElementState::Pressed;
                            self.mouse.right_button = state == ElementState::Pressed;
                        }
                        _ => {}
                    }
                    self.request_output();
                }
            }
            WindowEvent::RedrawRequested => {
                let mut changes = self.surface.clone().unwrap().take_changes();
                if let Some(update) = &mut self.update_fn {
                    changes |= update(&self.surface.clone().unwrap());
                }
                self.update_size(self.deferred_size);
                self.deferred_size = None;

                self.update_input();
                changes |= self.surface.clone().unwrap().take_changes();

                if changes.contains(NodeChanges::SLOTS) {
                    self.surface.clone().unwrap().negociate_slots();
                    changes |= self.surface.clone().unwrap().take_changes();
                }

                let drawn = if changes.contains(NodeChanges::OUTPUT) {
                    let surface = self.surface.clone().unwrap();
                    if let (Some(overlay), Some(window)) = (&mut self.overlay, &self.window) {
                        overlay.prepare(window, &surface);
                        surface.draw_with(|_, encoder, target| {
                            overlay.render(&surface, encoder, target)
                        })
                    } else {
                        surface.draw()
                    }
                } else {
                    false
                };

                if drawn && (self.poll_fn)() {
                    event_loop.exit();
                } else if self
                    .surface
                    .clone()
                    .unwrap()
                    .pending_changes()
                    .contains(NodeChanges::OUTPUT)
                {
                    let window = self.window.as_ref().unwrap();
                    window.request_redraw();
                }
            }
            _ => (),
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if self
            .surface
            .as_ref()
            .map(|surface| surface.pending_changes().contains(NodeChanges::OUTPUT))
            .unwrap_or(false)
        {
            let window = self.window.as_ref().unwrap();
            window.request_redraw();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn consumed_events_are_not_simulation_input() {
        assert!(!simulation_input_allowed(true));
        assert!(simulation_input_allowed(false));
    }

    #[test]
    fn escape_is_not_a_close_request() {
        assert!(!is_exit_event(false, false));
        assert!(is_exit_event(true, false));
        assert!(closes_window(&WindowEvent::CloseRequested));
    }
}

fn simulation_input_allowed(consumed: bool) -> bool {
    !consumed
}

fn closes_window(event: &WindowEvent) -> bool {
    is_exit_event(
        matches!(event, WindowEvent::CloseRequested),
        matches!(event, WindowEvent::Destroyed),
    )
}

fn is_exit_event(close_requested: bool, destroyed: bool) -> bool {
    close_requested || destroyed
}
