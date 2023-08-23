use cgmath::Vector3;
use vss::*;
use std::iter;
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
    vr_flows: Vec<Flow>,

    active: bool,
    static_pos: Option<(f32, f32)>,
    mouse: MouseInput,

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

        // Create vr flows.
        let mut vr_flows = Vec::new();
        vr_flows.resize_with(flow_count, Flow::new);

        Self {
            window,
            vr_flows,
            events_loop: Some(events_loop),
            active: false,
            static_pos,
            mouse: MouseInput {
                position: (0.0, 0.0),
                left_button: false,
                right_button: false,
            },
            varjo,
        }
    }

    pub fn window(&mut self) -> &mut winit::window::Window {
        return &mut self.window;
    }

    pub async fn run_and_exit<I, P>(mut self, mut init_fn: I, mut poll_fn: P)
    where
        I: 'static + FnMut(&mut WindowVRSurface, &mut Surface, Texture),
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
                    1,
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
                    1,
                )
                .await
            }
        };

        Varjo::check_handles(&surface);
        self.varjo.create_render_targets(&surface);

        let (vr_framebuffer_texture, _) = self.varjo.get_latest_render_target();
        init_fn(&mut self, &mut surface, vr_framebuffer_texture.as_texture());

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
                        WindowEvent::MouseInput { state, button, .. } => {
                            if self.active {
                                match button {
                                    MouseButton::Left => {
                                        self.mouse.left_button = *state == ElementState::Pressed;
                                    }
                                    MouseButton::Right => {
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
                    if self.varjo.begin_frame_sync() {
                        self.set_varjo_data();
                        self.draw_varjo(&surface);
                        self.varjo.end_frame();
                    }
                    surface.draw();
                }
                Event::RedrawEventsCleared => {
                    //*control_flow = ControlFlow::Exit;
                    self.window.request_redraw();
                }
                Event::MainEventsCleared => {
                    self.update_size(&mut surface, deferred_size);

                    self.update_input(&mut surface);

                    self.update_vr_input();

                    if poll_fn() {
                        *control_flow = ControlFlow::Exit;
                    }
                }
                _ => {}
            }
        });
    }

    fn set_varjo_data(&mut self) {
        let view_matrices = self.varjo.get_current_view_matrices();
        let proj_matrices = self.varjo.get_current_proj_matrices();
        // let head_position = 0.5 * (view_matrices[0].w.truncate() + view_matrices[1].w.truncate());
        let eye_position = Vector3::new(0.0, 0.0, 0.0);
        let (left_gaze, right_gaze, _focus_distance) = self.varjo.get_current_gaze();

        for (i, flow) in self.vr_flows.iter_mut().enumerate(){
            let mut eye = flow.eye_mut();
            eye.position = eye_position;
            eye.view = view_matrices[i];
            eye.proj = proj_matrices[i];
            eye.gaze = if i % 2 == 0 {left_gaze} else {right_gaze};
        }
    }

    pub fn draw_varjo(&mut self, surface: &Surface){
        let mut encoder = surface.device()
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Varjo Render Encoder"),
            });

        let (color_rt, _depth_rt) = self.varjo.get_current_render_target();

        self.vr_flows
            .iter()
            .for_each(|f| f.render(surface, &mut encoder, &color_rt));

        surface.queue().submit(iter::once(encoder.finish()));
        self.vr_flows.iter().for_each(|f| f.post_render(surface));
    }

    pub fn inspect(&self, inspector: &mut dyn Inspector) {
        for (i, flow) in self.vr_flows.iter().enumerate() {
            inspector.flow(i, &flow);
        }
    }

    pub fn add_node(&mut self, node: Box<dyn Node>, flow_index: usize) {
        self.vr_flows[flow_index].add_node(node);
    }

    pub fn negociate_slots(&self, surface: &Surface) {
        for flow in self.vr_flows.iter() {
            flow.negociate_slots(surface);
        }
    }

    fn update_input(&self, surface: &mut Surface) {
        for f in surface.flows.iter() {
            f.input(&self.mouse);
        }
    }
    
    fn update_vr_input(&self) {
        for f in self.vr_flows.iter() {
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
