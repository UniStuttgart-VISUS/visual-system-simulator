use crate::*;
use std::cell::RefCell;

/// A factory to create device objects.
pub type DeviceFactory = gfx_device_gl::Factory;

/// An encoder to manipulate a device command queue.
pub type DeviceEncoder = gfx::Encoder<gfx_device_gl::Resources, gfx_device_gl::CommandBuffer>;

/// Render Target Types of this Window.
pub type RenderTargetColor = gfx::handle::RenderTargetView<gfx_device_gl::Resources, ColorFormat>;
pub type RenderTargetDepthFormat = (gfx::format::D24_S8, gfx::format::Unorm);
pub type RenderTargetDepth = gfx::handle::DepthStencilView<gfx_device_gl::Resources, RenderTargetDepthFormat>;

/// Represents a window along with its associated rendering context and [Flow].
pub struct Window {
    windowed_context: glutin::WindowedContext<glutin::PossiblyCurrent>,
    events_loop: RefCell<glutin::EventsLoop>,
    device: RefCell<gfx_device_gl::Device>,
    factory: RefCell<DeviceFactory>,
    encoder: RefCell<DeviceEncoder>,

    render_target: RefCell<RenderTargetColor>,
    main_depth: RefCell<RenderTargetDepth>,

    last_head: RefCell<Head>,
    last_gaze: RefCell<Gaze>,
    active: RefCell<bool>,
    values: RefCell<ValueMap>,

    remote: Option<Remote>,
    flow: Flow,
    vis_param: RefCell<VisualizationParameters>
}

impl Window {
    pub fn new(visible: bool, remote: Option<Remote>, values: ValueMap) -> Self {
        // Create a window and context.
        let gl_version = glutin::GlRequest::GlThenGles {
            opengles_version: (3, 2),
            opengl_version: (4, 1),
        };
        let events_loop = glutin::EventsLoop::new();
        let window_builder = glutin::WindowBuilder::new()
            .with_title("Visual System Simulator")
            .with_min_dimensions(glutin::dpi::LogicalSize::new(640.0, 360.0))
            .with_dimensions(glutin::dpi::LogicalSize::new(1280.0, 720.0))
            .with_visibility(visible);
        let context_builder = glutin::ContextBuilder::new()
            .with_vsync(true)
            .with_gl(gl_version);
        let (windowed_context, mut device, mut factory, render_target, main_depth) =
            gfx_window_glutin::init::<
                (gfx::format::R8_G8_B8_A8, gfx::format::Unorm),
                gfx::format::DepthStencil,
            >(window_builder, context_builder, &events_loop)
            .unwrap();

        windowed_context.window().hide_cursor(true);

        // Create a command buffer.
        let encoder: DeviceEncoder = factory.create_command_buffer().into();

        unsafe {
            device.with_gl(|gl| gl.Disable(gfx_gl::FRAMEBUFFER_SRGB));
        }

        Window {
            flow: Flow::new(),
            remote,
            windowed_context,
            events_loop: RefCell::new(events_loop),
            device: RefCell::new(device),
            factory: RefCell::new(factory),
            encoder: RefCell::new(encoder),
            render_target: RefCell::new(render_target),
            main_depth: RefCell::new(main_depth),
            last_head: RefCell::new(Head {
                yaw: 0.0,
                pitch: 0.0,
            }),
            last_gaze: RefCell::new(Gaze { x: 0.5, y: 0.5 }),
            active: RefCell::new(false),
            values: RefCell::new(values),
            vis_param: RefCell::new(VisualizationParameters::default())
        }
    }
}

impl Window {
    pub fn add_node(&mut self, node: Box<dyn Node>) {
        self.flow.add_node(node);
    }

    pub fn replace_node(&mut self, index: usize, node: Box<dyn Node>) {
        self.flow.replace_node(index, node);
    }

    pub fn nodes_len(&self) -> usize {
        self.flow.nodes_len()
    }

    pub fn update_nodes(&mut self) {
        self.flow.negociate_slots(&self);
        self.flow.update_values(&self, &self.values.borrow());
    }

    pub fn set_values(&self, values: ValueMap) {
        self.values.replace(values);
        self.flow.update_values(&self, &self.values.borrow());
    }

    pub fn factory(&self) -> &RefCell<DeviceFactory> {
        &self.factory
    }

    pub fn encoder(&self) -> &RefCell<DeviceEncoder> {
        &self.encoder
    }

    pub fn flush(&self, encoder: &mut DeviceEncoder) {
        use std::ops::DerefMut;
        let mut device = self.device.borrow_mut();
        encoder.flush(device.deref_mut());
    }

    fn override_gaze(gaze: Gaze, values: &ValueMap) -> Gaze {
        if let (Some(gaze_x), Some(gaze_y)) = (values.get("gaze_x"), values.get("gaze_y")) {
            Gaze {
                x: gaze_x.as_f64().unwrap_or(0.0) as f32,
                y: gaze_y.as_f64().unwrap_or(0.0) as f32,
            }
        } else {
            gaze
        }
    }

    pub fn target(&self) -> gfx::handle::RenderTargetView<gfx_device_gl::Resources, ColorFormat> {
        self.render_target.borrow().clone()
    }

    pub fn replace_targets(&self, target_color: RenderTargetColor, target_depth: RenderTargetDepth) {
        self.render_target.replace(target_color);
        self.main_depth.replace(target_depth);
    }

    pub fn poll_events(&self) -> bool {
        let mut done = false;
        let mut deferred_size = None;
        let mut deferred_gaze =
            Self::override_gaze(self.last_gaze.borrow().clone(), &self.values.borrow());

        // Poll for window events.
        self.events_loop.borrow_mut().poll_events(|event| {
            if let glutin::Event::WindowEvent { event, .. } = event {
                match event {
                    glutin::WindowEvent::KeyboardInput {
                        input:
                            glutin::KeyboardInput {
                                state: glutin::ElementState::Pressed,
                                virtual_keycode: Some(glutin::VirtualKeyCode::A),
                                ..
                            },
                        ..
                    } => {
                        self.vis_param.borrow_mut().dir_calc_scale+=0.5;
                    },
                    glutin::WindowEvent::KeyboardInput {
                        input:
                            glutin::KeyboardInput {
                                state: glutin::ElementState::Pressed,
                                virtual_keycode: Some(glutin::VirtualKeyCode::D),
                                ..
                            },
                        ..
                    } => {
                        self.vis_param.borrow_mut().dir_calc_scale-=0.5;
                    },
                    glutin::WindowEvent::KeyboardInput {
                        input:
                            glutin::KeyboardInput {
                                state: glutin::ElementState::Pressed,
                                virtual_keycode: Some(glutin::VirtualKeyCode::W),
                                ..
                            },
                        ..
                    } => {
                        self.vis_param.borrow_mut().heat_scale+=0.5;
                    },
                    glutin::WindowEvent::KeyboardInput {
                        input:
                            glutin::KeyboardInput {
                                state: glutin::ElementState::Pressed,
                                virtual_keycode: Some(glutin::VirtualKeyCode::S),
                                ..
                            },
                        ..
                    } => {
                        self.vis_param.borrow_mut().heat_scale-=0.5;
                    },
                    glutin::WindowEvent::KeyboardInput {
                        input:
                            glutin::KeyboardInput {
                                virtual_keycode: Some(glutin::VirtualKeyCode::Key0),
                                ..
                            },
                        ..
                    } => {
                        self.vis_param.borrow_mut().vis_type=VisualizationType::Output;
                    },
                    glutin::WindowEvent::KeyboardInput {
                        input:
                            glutin::KeyboardInput {
                                virtual_keycode: Some(glutin::VirtualKeyCode::Key1),
                                ..
                            },
                        ..
                    } => {
                        self.vis_param.borrow_mut().vis_type=VisualizationType::Deflection;
                    },
                    glutin::WindowEvent::KeyboardInput {
                        input:
                            glutin::KeyboardInput {
                                virtual_keycode: Some(glutin::VirtualKeyCode::Escape),
                                ..
                            },
                        ..
                    }
                    | glutin::WindowEvent::CloseRequested
                    | glutin::WindowEvent::Destroyed => done = true,
                    glutin::WindowEvent::Focused(active) => {
                        self.active.replace(active);
                    }
                    glutin::WindowEvent::Resized(size) => {
                        deferred_size = Some(size);
                    }
                    glutin::WindowEvent::CursorMoved { position, .. } => {
                        if *self.active.borrow() {
                            let window_size =
                                &self.windowed_context.window().get_inner_size().unwrap();
                            deferred_gaze = Self::override_gaze(
                                Gaze {
                                    x: position.x as f32 / window_size.width as f32,
                                    y: 1.0 - (position.y as f32 / window_size.height as f32),
                                },
                                &self.values.borrow(),
                            );
                            let last_head = &mut self.last_head.borrow_mut();
                            last_head.yaw = position.x as f32 / window_size.width as f32
                                * std::f32::consts::PI
                                * 2.0
                                - 0.5;
                            last_head.pitch = position.y as f32 / window_size.height as f32
                                * std::f32::consts::PI
                                - 0.5;
                        }
                    }
                    glutin::WindowEvent::CursorLeft { .. } => {
                        if *self.active.borrow() {
                            deferred_gaze =
                                Self::override_gaze(Gaze { x: 0.5, y: 0.5 }, &self.values.borrow());
                        }
                    }
                    _ => (),
                }
            }
        });

        if let Some(size) = deferred_size {
            // Update pipline IO.
            let dpi_factor = self.windowed_context.window().get_hidpi_factor();
            let size = size.to_physical(dpi_factor);
            self.windowed_context.resize(size);
            gfx_window_glutin::update_views(
                &self.windowed_context,
                &mut self.render_target.borrow_mut(),
                &mut self.main_depth.borrow_mut(),
            );
            self.flow.negociate_slots(&self);
            self.flow.update_values(&self, &self.values.borrow());
        }

        // Update input.
        self.flow.input(&self.last_head.borrow(), &deferred_gaze, &self.vis_param.borrow());
        *self.last_gaze.borrow_mut() = deferred_gaze;

        self.encoder
            .borrow_mut()
            .clear(&self.render_target.borrow(), [68.0 / 255.0; 4]);
        self.flow.render(&self);

        use gfx::Device;
        self.flush(&mut self.encoder().borrow_mut());
        self.device.borrow_mut().cleanup();

        self.windowed_context.swap_buffers().unwrap();

        if let Some(remote) = &self.remote {
            remote.send_frame();
        }

        return done;
    }
}
