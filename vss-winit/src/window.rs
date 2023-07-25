use vss::*;
use cgmath::{Matrix4, SquareMatrix, Vector4};
use winit::{
    dpi::*,
    event::*,
    event_loop::{ControlFlow, EventLoop},
    platform::run_return::EventLoopExtRunReturn,
    window::WindowBuilder,
};

/// Represents a window along with its associated rendering context and [Flow].
pub struct Window {
    wgpu_window: winit::window::Window,
    events_loop: EventLoop<()>,
    pub surface: Surface,

    active: bool,
    static_pos: Option<(f32, f32)>,
    mouse: MouseInput,

    override_gaze: bool,
    override_view: bool,
}

impl Window {
    pub async fn new(visible: bool, flow_count: usize, static_pos: Option<(f32, f32)>) -> Self {
        // Create a window and context.
        let events_loop = EventLoop::new();

        let window_builder = WindowBuilder::new()
            .with_title("Visual System Simulator")
            .with_min_inner_size(LogicalSize::new(640.0, 360.0))
            .with_inner_size(LogicalSize::new(1280.0, 720.0))
            .with_visible(visible);

        // let context_builder = glutin::ContextBuilder::new()
        //     .with_vsync(true)
        //     .with_gl(gl_version);
        // let (wgpu_window, mut device, mut factory, render_target, main_depth) =
        //     gfx_window_glutin::init::<
        //         (gfx::format::R8_G8_B8_A8, gfx::format::Unorm),
        //         gfx::format::DepthStencil,
        //     >(window_builder, context_builder, &events_loop)
        //     .unwrap();

        let wgpu_window = window_builder.build(&events_loop).unwrap();
        wgpu_window.set_cursor_visible(true);
        let window_size = wgpu_window.inner_size();

        let surface = Surface::new(
            [window_size.width, window_size.height],
            &wgpu_window,
            flow_count,
        )
        .await;

        Window {
            wgpu_window,
            events_loop,
            surface,
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

    pub fn poll_events(&mut self) -> bool {
        let mut done = false;
        let mut deferred_size = None;
        let mut redraw_requested = true;

        // Poll for window events.
        // TODO-WGPU use .run() instead of .run_return() as it is highly discouraged and incompatible with some platforms
        self.events_loop.run_return(|event, _, control_flow| {
            match event {
                Event::WindowEvent {
                    window_id,
                    ref event,
                } if window_id == self.wgpu_window.id() => {
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
                            done = true;
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
                Event::RedrawRequested(window_id) if window_id == self.wgpu_window.id() => {
                    redraw_requested = true;
                }
                Event::RedrawEventsCleared => {
                    *control_flow = ControlFlow::Exit;
                    self.wgpu_window.request_redraw();
                }
                _ => {}
            }
        });

        if self.static_pos.is_some() {
            // Update flow IO.
            let new_size = PhysicalSize::new(1920, 1080);
            self.surface.resize([new_size.width, new_size.height]);
            // TODO-WGPU
            // for (i, f) in self.flow.iter().enumerate(){
            //     f.negociate_slots(&self);
            //     f.last_perspective.borrow_mut().proj = cgmath::perspective(
            //         cgmath::Deg(70.0), (size.width/size.height) as f32, 0.05, 1000.0);
            // }
        }

        if let Some(new_size) = deferred_size {
            // Update flow IO.
            // let dpi_factor = self.wgpu_window.scale_factor();
            // let size = size.to_physical(dpi_factor);
            self.surface.resize([new_size.width, new_size.height]);
            // TODO-WGPU
            // for (i, f) in self.flow.iter().enumerate(){
            //     f.negociate_slots(&self);
            //     f.last_perspective.borrow_mut().proj = cgmath::perspective(
            //         cgmath::Deg(70.0), (size.width/size.height) as f32, 0.05, 1000.0);
            // }
        }

        // Update input.
        for f in self.surface.flows.iter() {
            if self.override_view || self.override_gaze {
                let view_pos = self.static_pos.unwrap_or(self.mouse.position);

                let yaw =
                    view_pos.0 / (self.surface.width() as f32) * std::f32::consts::PI * 2.0 - 0.5;
                let pitch =
                    view_pos.1 / (self.surface.height() as f32) * std::f32::consts::PI - 0.5; //50 mm lens
                let view = Matrix4::from_angle_x(cgmath::Rad(pitch))
                    * Matrix4::from_angle_y(cgmath::Rad(yaw));

                let mut eye = f.eye_mut();

                if self.override_view {
                    if !self.override_gaze {
                        eye.gaze =
                            (view * eye.view.invert().unwrap() * eye.gaze.extend(1.0)).truncate();
                    }
                    eye.view = view;
                }
                if self.override_gaze {
                    eye.gaze = (eye.view * view.invert().unwrap() * Vector4::unit_z()).truncate();
                }
            }
            f.input(&self.mouse);
        }

        if redraw_requested {
            self.surface.draw();
        }

        done
    }
}
