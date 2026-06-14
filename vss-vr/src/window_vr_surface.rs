use std::iter;
use std::rc::Rc;
use std::sync::Arc;

use vss::*;
use winit::{
    application::ApplicationHandler,
    dpi::*,
    error::EventLoopError,
    event::*,
    event_loop::{ActiveEventLoop, EventLoop},
    keyboard::{Key, NamedKey},
    window::Window,
};

use crate::Varjo;

/// Represents a window along with its associated rendering context and [Flow].
pub struct WindowVRSurface {
    surface: Option<Rc<Surface<'static>>>,
    window: Option<Arc<Window>>,
    flow_count: usize,
    vr_flows: Vec<Flow>,

    deferred_size: Option<PhysicalSize<u32>>,
    visible: bool,
    init_fn: Option<Box<InitFn>>,
    poll_fn: Box<dyn FnMut() -> bool>,

    active: bool,
    static_pos: Option<(f32, f32)>,
    mouse: MouseInput,

    varjo: Varjo,
}

type InitFn = dyn FnMut(&mut WindowVRSurface, &mut Surface, Texture);

impl WindowVRSurface {
    pub fn new<I, P>(
        visible: bool,
        flow_count: usize,
        static_pos: Option<(f32, f32)>,
        varjo: Varjo,
        init_fn: I,
        poll_fn: P,
    ) -> Self
    where
        I: 'static + FnMut(&mut WindowVRSurface, &mut Surface, Texture),
        P: 'static + FnMut() -> bool,
    {
        let mut vr_flows = Vec::new();
        vr_flows.resize_with(flow_count, Flow::new);

        Self {
            surface: None,
            window: None,
            flow_count,
            vr_flows,
            deferred_size: None,
            visible,
            init_fn: Some(Box::new(init_fn)),
            poll_fn: Box::new(poll_fn),
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

    pub fn run_and_exit(mut self) -> Result<(), EventLoopError> {
        let event_loop = EventLoop::new().unwrap();
        event_loop.run_app(&mut self)
    }

    fn set_varjo_data(&mut self) {
        let view_matrices = self.varjo.get_current_view_matrices();
        let proj_matrices = self.varjo.get_current_proj_matrices();
        let head_position = 0.5 * (view_matrices[0].w.truncate() + view_matrices[1].w.truncate());
        let (left_gaze, right_gaze, _focus_distance) = self.varjo.get_current_gaze();

        for (i, flow) in self.vr_flows.iter_mut().enumerate() {
            let mut eye = flow.eye_mut();
            eye.position = head_position;
            eye.view = view_matrices[i];
            eye.proj = proj_matrices[i];
            eye.gaze = if i % 2 == 0 { left_gaze } else { right_gaze };
        }
    }

    pub fn draw_varjo(&mut self, surface: &Surface) {
        let mut encoder =
            surface
                .device()
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

    fn update_input(&self) {
        let surface = self.surface.clone().unwrap();
        for f in surface.flows.iter() {
            f.input(&self.mouse);
        }
    }

    fn update_vr_input(&self) {
        for f in self.vr_flows.iter() {
            f.input(&self.mouse);
        }
    }

    fn update_size(&mut self, deferred_size: Option<PhysicalSize<u32>>) {
        let new_size = if self.static_pos.is_some() {
            Some(PhysicalSize::new(1920, 1080))
        } else {
            deferred_size
        };

        if let Some(new_size) = new_size {
            if let Some(surface) = &mut self.surface {
                let surface = Rc::get_mut(surface).unwrap();
                surface.resize([new_size.width, new_size.height]);
            }
        }
    }
}

impl ApplicationHandler for WindowVRSurface {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let window_attributes = Window::default_attributes()
            .with_title("Visual System Simulator")
            .with_min_inner_size(LogicalSize::new(640.0, 360.0))
            .with_inner_size(LogicalSize::new(1280.0, 720.0))
            .with_visible(self.visible);

        let window = Arc::new(event_loop.create_window(window_attributes).unwrap());
        window.set_cursor_visible(true);
        let window_size = window.inner_size();

        let mut surface = match self.varjo.create_custom_vk_instance() {
            Some(instance) => {
                let surface = instance.create_surface(window.clone()).unwrap();
                let adapter =
                    pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
                        power_preference: wgpu::PowerPreference::HighPerformance,
                        compatible_surface: Some(&surface),
                        force_fallback_adapter: false,
                    }))
                    .expect("Cannot create adapter");

                let (device, queue) = self.varjo.create_custom_vk_device(&instance, &adapter);

                pollster::block_on(Surface::with_existing(
                    [window_size.width, window_size.height],
                    self.flow_count,
                    surface,
                    adapter,
                    device,
                    queue,
                ))
            }
            None => Surface::new(
                [window_size.width, window_size.height],
                window.clone(),
                self.flow_count,
            ),
        };

        Varjo::check_handles(&surface);
        self.varjo.create_render_targets(&surface);

        let (vr_framebuffer_texture, _) = self.varjo.get_latest_render_target();
        let mut init_fn = self.init_fn.take().unwrap();
        init_fn(self, &mut surface, vr_framebuffer_texture.as_texture());
        self.init_fn = Some(init_fn);

        window.request_redraw();
        self.window = Some(window);
        self.surface = Some(Rc::new(surface));
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        if self.window.as_ref().map(|window| window.id()) != Some(window_id) {
            return;
        }

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
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
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
                            self.mouse.left_button = state == ElementState::Pressed;
                        }
                        MouseButton::Right => {
                            self.mouse.right_button = state == ElementState::Pressed;
                        }
                        _ => {}
                    }
                }
            }
            WindowEvent::RedrawRequested => {
                let surface = self.surface.clone().unwrap();
                if self.varjo.begin_frame_sync() {
                    self.set_varjo_data();
                    self.draw_varjo(&surface);
                    self.varjo.end_frame();
                }
                surface.draw();
            }
            _ => {}
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
        self.update_vr_input();

        if let Some(window) = &self.window {
            window.request_redraw();
        }

        if (self.poll_fn)() {
            event_loop.exit();
        }
    }
}
