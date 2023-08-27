use cgmath::{Matrix4, SquareMatrix, Vector4};
use vss::*;
use winit::{
    dpi::*,
    event::*,
    event_loop::{ControlFlow, EventLoop, EventLoopWindowTarget},
    window::WindowBuilder,
};

/// Represents a window along with its associated rendering context and [Flow].
pub struct WindowSurface {
    events_loop: Option<EventLoop<()>>,
    window: winit::window::Window,
    flow_count: usize,

    active: bool,
    static_pos: Option<(f32, f32)>,
    mouse: MouseInput,

    override_gaze: bool,
    override_view: bool,
}

impl WindowSurface {
    pub fn new(visible: bool, flow_count: usize, static_pos: Option<(f32, f32)>) -> Self {
        let window_builder = WindowBuilder::new()
            .with_title("Visual System Simulator")
            .with_min_inner_size(LogicalSize::new(640.0, 360.0))
            .with_inner_size(LogicalSize::new(1280.0, 720.0))
            .with_visible(visible);

        let events_loop = EventLoop::new();
        let window = window_builder.build(&events_loop).unwrap();
        window.set_cursor_visible(true);

        Self {
            window,
            events_loop: Some(events_loop),
            flow_count,
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

    pub fn window(&mut self) -> &mut winit::window::Window {
        &mut self.window
    }

    pub async fn run_and_exit<I, P>(mut self, init_fn: I, mut poll_fn: P)
    where
        I: 'static + FnOnce(&mut Surface),
        P: 'static + FnMut() -> bool,
    {
        let window_size = self.window.inner_size();

        let mut surface = Surface::new(
            [window_size.width, window_size.height],
            &self.window,
            self.flow_count,
        )
        .await;

        init_fn(&mut surface);

        let events_loop = self.events_loop.take().unwrap();
        let mut deferred_size = None;

        let event_handler = move |event: Event<'_, ()>,
                                  _: &EventLoopWindowTarget<()>,
                                  control_flow: &mut ControlFlow| {
            *control_flow = ControlFlow::Poll;
            match event {
                Event::WindowEvent {
                    window_id,
                    ref event,
                } if window_id == self.window.id() => {
                    match event {
                        WindowEvent::KeyboardInput {
                            input:
                                KeyboardInput {
                                    virtual_keycode: Some(VirtualKeyCode::Escape),
                                    ..
                                },
                            ..
                        }
                        | WindowEvent::CloseRequested
                        | WindowEvent::Destroyed => {
                            *control_flow = ControlFlow::Exit;
                        }
                        WindowEvent::Focused(active) => {
                            self.active = *active;
                        }
                        WindowEvent::Resized(size) => {
                            deferred_size = Some(*size);
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
                                        self.override_view = *state == ElementState::Pressed;
                                        self.mouse.left_button = *state == ElementState::Pressed;
                                    }
                                    MouseButton::Right => {
                                        self.override_gaze = *state == ElementState::Pressed;
                                        self.mouse.right_button = *state == ElementState::Pressed;
                                    }
                                    _ => {}
                                }
                            }
                        }
                        _ => (),
                    }
                }
                Event::RedrawRequested(window_id) if window_id == self.window.id() => {
                    surface.draw();
                }
                Event::RedrawEventsCleared => {
                    //*control_flow = ControlFlow::Exit;
                    self.window.request_redraw();
                }
                Event::MainEventsCleared => {
                    self.update_size(&mut surface, deferred_size);

                    self.update_input(&mut surface);

                    if poll_fn() {
                        *control_flow = ControlFlow::Exit;
                    }
                }
                _ => {}
            }
        };

        #[cfg(target_arch = "wasm32")]
        {
            use winit::platform::web::EventLoopExtWebSys;
            events_loop.spawn(event_handler);
        }
        #[cfg(not(target_arch = "wasm32"))]
        events_loop.run(event_handler);
    }

    fn update_input(&self, surface: &mut Surface) {
        for f in surface.flows.iter() {
            if self.override_view || self.override_gaze {
                let view_pos = self.static_pos.unwrap_or(self.mouse.position);

                let yaw = (view_pos.0 / (surface.width() as f32) - 0.5) * std::f32::consts::PI * 2.0;
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

    fn update_size(&mut self, surface: &mut Surface, deferred_size: Option<PhysicalSize<u32>>) {
        if self.static_pos.is_some() {
            // Update flow IO.
            let new_size = PhysicalSize::new(1920, 1080);
            surface.resize([new_size.width, new_size.height]);
            // TODO-WGPU
            // for (i, f) in self.flow.iter().enumerate(){
            //     f.negociate_slots(&self);
            //     f.last_perspective.borrow_mut().proj = cgmath::perspective(
            //         cgmath::Deg(70.0), (size.width/size.height) as f32, 0.05, 1000.0);
            // }
        }

        if let Some(new_size) = deferred_size {
            // Update flow IO.
            // let dpi_factor = self.window.scale_factor();
            // let size = size.to_physical(dpi_factor);
            surface.resize([new_size.width, new_size.height]);
            // TODO-WGPU
            // for (i, f) in self.flow.iter().enumerate(){
            //     f.negociate_slots(&self);
            //     f.last_perspective.borrow_mut().proj = cgmath::perspective(
            //         cgmath::Deg(70.0), (size.width/size.height) as f32, 0.05, 1000.0);
            // }
        }
    }
}
