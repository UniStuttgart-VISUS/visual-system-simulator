use cgmath::{Matrix4, SquareMatrix, Vector4, Vector3};
use vss::*;
use winit::{
    dpi::*,
    event::*,
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

use crate::Varjo;

/// Represents a window along with its associated rendering context and [Flow].
pub struct WindowVRSurface {
    events_loop: Option<EventLoop<()>>,
    window: winit::window::Window,
    flow_count: usize,

    active: bool,
    static_pos: Option<(f32, f32)>,
    mouse: MouseInput,

    override_gaze: bool,
    override_view: bool,

    varjo: Varjo,
}

impl WindowVRSurface {
    pub fn new(visible: bool, flow_count: usize, static_pos: Option<(f32, f32)>, varjo: Varjo) -> Self {
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
            varjo,
        }
    }

    pub fn window(&mut self) -> &mut winit::window::Window {
        return &mut self.window;
    }

    pub async fn run_and_exit<I, P>(mut self, mut init_fn: I, mut poll_fn: P)
    where
        I: 'static + FnMut(&mut Surface),
        P: 'static + FnMut() -> bool,
    {
        let window_size = self.window.inner_size();

        let instance = self.varjo.create_custom_vk_instance();

        let mut surface = match instance {
            Some(inst) => {
                println!("Surface creation using Custom Instance");
                let surface: wgpu::Surface = unsafe { inst.create_surface(&self.window) }.unwrap();
                println!("Surface created");
                let adapter: wgpu::Adapter = inst
                    .request_adapter(&wgpu::RequestAdapterOptions {
                        power_preference: wgpu::PowerPreference::HighPerformance,
                        compatible_surface: Some(&surface),
                        force_fallback_adapter: false,
                    })
                    .await
                    .expect("Cannot create adapter");
                println!("Adapter created");

                let (device, queue) = self.varjo.create_custom_vk_device(&inst, &adapter);
                println!("Device and Queue created");
        
                Surface::with_existing(
                    [window_size.width, window_size.height],
                    self.flow_count,
                    surface,
                    adapter,
                    device,
                    queue,
                )
                .await
            },
            _ => {
                Surface::new(
                    [window_size.width, window_size.height],
                    &self.window,
                    self.flow_count,
                )
                .await
            }
        };

        Varjo::check_handles(&surface);
        self.varjo.create_render_targets(&surface);

        init_fn(&mut surface);

        let events_loop = self.events_loop.take().unwrap();
        let mut deferred_size = None;

        events_loop.run(move |event, _, control_flow| {
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
                    // TODO: somehow separate varjo drawing from the surface and only draw the final framebuffer onto the surface (if possible with gui node)
                    surface.draw();
                    if self.varjo.begin_frame_sync() {
                        self.set_varjo_data(&mut surface);
                        self.varjo.draw(&surface);
                        self.varjo.end_frame();
                    }
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
        });
    }

    fn set_varjo_data(&self, surface: &mut Surface) {
        let view_matrices = self.varjo.get_current_view_matrices();
        let proj_matrices = self.varjo.get_current_proj_matrices();
        // let head_position = 0.5 * (view_matrices[0].w.truncate() + view_matrices[1].w.truncate());
        let eye_position = Vector3::new(0.0, 0.0, 0.0);
        let (left_gaze, right_gaze, _focus_distance) = self.varjo.get_current_gaze();

        for (i, flow) in surface.flows.iter_mut().enumerate(){
            let mut eye = flow.eye_mut();
            eye.position = eye_position;
            eye.view = view_matrices[i];
            eye.proj = proj_matrices[i];
            eye.gaze = if i % 2 == 0 {left_gaze} else {right_gaze};
        }
    }

    fn update_input(&self, surface: &mut Surface) {
        for f in surface.flows.iter() {
            if self.override_view || self.override_gaze {
                let view_pos = self.static_pos.unwrap_or(self.mouse.position);

                let yaw = view_pos.0 / (surface.width() as f32) * std::f32::consts::PI * 2.0 - 0.5;
                let pitch = view_pos.1 / (surface.height() as f32) * std::f32::consts::PI - 0.5; //50 mm lens
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
