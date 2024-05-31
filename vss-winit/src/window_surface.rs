use std::rc::Rc;

use cgmath::{Matrix4, SquareMatrix, Vector4};
use vss::*;
use winit::{
    application::ApplicationHandler, dpi::*, error::EventLoopError, event::*, event_loop::{ActiveEventLoop, EventLoop}, keyboard::{Key, NamedKey}, window::Window
};

/// Represents a window along with its associated rendering context and [Flow].
pub struct WindowSurface {
    surface: Option<Rc<Surface<'static>>>,
    flow_count: usize,

    deferred_size: Option<PhysicalSize<u32>>,
    visible: bool,
    init_fn: Box<dyn Fn(&mut Surface)>,
    poll_fn: Box<dyn FnMut() -> bool>,

    active: bool,
    static_pos: Option<(f32, f32)>,
    mouse: MouseInput,

    override_gaze: bool,
    override_view: bool,
}

impl WindowSurface {
    pub fn new<I, P>(
        visible: bool,
        flow_count: usize,
        static_pos: Option<(f32, f32)>,
        init_fn: I,
        poll_fn: P,
    ) -> Self
    where
        I: 'static + Fn(&mut Surface),
        P: 'static + FnMut() -> bool,
    {
        Self {
            surface: None,
            flow_count,
            deferred_size: None,
            visible: visible,
            init_fn: Box::new(init_fn),
            poll_fn: Box::new(poll_fn),
            active: false,
            static_pos,
            mouse: MouseInput {
                position: (0.0, 0.0),
                left_button: false,
                right_button: false,
            },
            override_view: static_pos.is_some(),
            override_gaze: false,
        }
    }

    pub fn run_app(mut self) -> Result<(), EventLoopError> {
        let event_loop = EventLoop::new().unwrap();
    
        #[cfg(target_arch = "wasm32")]
        {
            use winit::platform::web::EventLoopExtWebSys;
            event_loop.spawn_app(event_handler)
        }
        #[cfg(not(target_arch = "wasm32"))]
        event_loop.run_app(&mut self)
    }
    

    fn update_input(&self) {
        let surface = self.surface.clone().unwrap();
        for f in surface.flows.iter() {
            if self.override_view || self.override_gaze {
                let view_pos = self.static_pos.unwrap_or(self.mouse.position);

                let yaw =
                    (view_pos.0 / (surface.width() as f32) - 0.5) * std::f32::consts::PI * 2.0;
                let pitch = (view_pos.1 / (surface.height() as f32) - 0.5) * std::f32::consts::PI; //50 mm lens
                let view = Matrix4::from_angle_x(cgmath::Rad(pitch))
                    * Matrix4::from_angle_y(cgmath::Rad(yaw));

                let mut eye = f.eye_mut();

                if self.override_view {
                    eye.view = view;
                }
                if self.override_gaze {
                    eye.gaze = (eye.view * view.invert().unwrap() * Vector4::unit_z()).truncate();
                }
            }
            f.input(&self.mouse);
        }
    }

    fn update_size(&mut self, deferred_size: Option<PhysicalSize<u32>>) {
        let new_size = if self.static_pos.is_some() {
            Some(PhysicalSize::new(1920, 1080))
        } else {
            // TODO-WGPU
            // let dpi_factor = self.window.scale_factor();
            // let size = size.to_physical(dpi_factor);
            deferred_size
        };

        if let Some(new_size) = new_size {
            if let Some(surface) = &mut self.surface {
                let surface = Rc::get_mut(surface).unwrap();

                surface.resize([new_size.width, new_size.height]);
                for flow in surface.flows.iter() {
                    flow.negociate_slots(&surface);
                    // TODO-WGPU
                    // flow.last_perspective.borrow_mut().proj = cgmath::perspective(
                    //    cgmath::Deg(70.0), (size.width/size.height) as f32, 0.05, 1000.0);
                }
            }
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

        let window = event_loop.create_window(window_attributes).unwrap();
        window.set_cursor_visible(true);
        let window_size = window.inner_size();

        let mut surface = Surface::new(
            [window_size.width, window_size.height],
            window,
            self.flow_count,
        );

        (self.init_fn)(&mut surface);

        surface.negociate_slots();
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
            }
            WindowEvent::CursorMoved { position, .. } => {
                if self.active {
                    self.mouse.position = (position.x as f32, position.y as f32);
                }
            }
            WindowEvent::CursorLeft { .. } => {
                if self.active {
                    self.override_view = false;
                    self.override_gaze = false;
                    //XXX: reset gaze?
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
                }
            }
            WindowEvent::RedrawRequested => {
                self.surface.clone().unwrap().draw();
            }
            _ => (),
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        self.update_size(self.deferred_size);
        self.deferred_size = None;

        let surface = self.surface.clone().unwrap();
        if !surface.validate_slots() {
            surface.negociate_slots();
        }

        self.update_input();

        if (self.poll_fn)() {
            event_loop.exit();
        }
    }
}
