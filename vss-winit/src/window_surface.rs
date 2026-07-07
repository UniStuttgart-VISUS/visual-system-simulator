use std::rc::Rc;
use std::sync::{Arc, RwLock};

use cgmath::{Matrix4, Rad, SquareMatrix, Vector3, Vector4};
use vss::*;
use winit::{
    application::ApplicationHandler,
    dpi::*,
    error::EventLoopError,
    event::*,
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    keyboard::{Key, NamedKey},
    platform::run_on_demand::EventLoopExtRunOnDemand,
    window::Window,
};

/// Represents a window along with its associated rendering context and [Flow].
pub struct WindowSurface {
    surface: Option<Rc<Surface<'static>>>,
    window: Option<Arc<Window>>,
    flow_count: usize,

    deferred_size: Option<PhysicalSize<u32>>,
    visible: bool,
    init_fn: Box<dyn Fn(&mut Surface)>,
    poll_fn: Box<dyn FnMut() -> bool>,

    active: bool,
    static_view: Option<(f32, f32)>,
    static_gaze: Option<(f32, f32)>,
    pose_input_size: Arc<RwLock<Option<[u32; 2]>>>,
    mouse: MouseInput,

    override_gaze: bool,
    override_view: bool,
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
        I: 'static + Fn(&mut Surface),
        P: 'static + FnMut() -> bool,
    {
        Self {
            surface: None,
            window: None,
            flow_count,
            deferred_size: None,
            visible: visible,
            init_fn: Box::new(init_fn),
            poll_fn: Box::new(poll_fn),
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
        }
    }

    pub fn run_app(mut self, event_loop: &mut EventLoop<()>) -> Result<(), EventLoopError> {
        event_loop.set_control_flow(ControlFlow::Wait);

        #[cfg(target_arch = "wasm32")]
        {
            use winit::platform::web::EventLoopExtWebSys;
            event_loop.spawn_app(self)
        }
        #[cfg(not(target_arch = "wasm32"))]
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
        window.set_cursor_visible(true);
        let window_size = window.inner_size();

        let mut surface = Surface::new(
            [window_size.width, window_size.height],
            window.clone(),
            self.flow_count,
        );

        (self.init_fn)(&mut surface);
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
        match event {
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        logical_key: Key::Named(NamedKey::Escape),
                        ..
                    },
                ..
            }
            | WindowEvent::CloseRequested
            | WindowEvent::Destroyed => {
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
                if self.active {
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
                if self.active {
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
                self.update_size(self.deferred_size);
                self.deferred_size = None;

                self.update_input();
                changes |= self.surface.clone().unwrap().take_changes();

                if changes.contains(NodeChanges::SLOTS) {
                    self.surface.clone().unwrap().negociate_slots();
                    changes |= self.surface.clone().unwrap().take_changes();
                }

                let drawn = if changes.contains(NodeChanges::OUTPUT) {
                    self.surface.clone().unwrap().draw()
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
