use std::cell::RefCell;

use glutin::dpi::*;
use glutin::GlRequest;

use super::*;
use crate::pipeline::*;

pub type DepthFormat = gfx::format::DepthStencil;

/// A device for window and context creation.
pub struct Window {
    remote: Option<Remote>,
    pipeline: Pipeline,

    windowed_context: glutin::WindowedContext<glutin::PossiblyCurrent>,
    events_loop: RefCell<glutin::EventsLoop>,
    device: RefCell<gfx_device_gl::Device>,
    factory: RefCell<gfx_device_gl::Factory>,
    encoder: RefCell<gfx::Encoder<gfx_device_gl::Resources, gfx_device_gl::CommandBuffer>>,

    render_target: RefCell<gfx::handle::RenderTargetView<gfx_device_gl::Resources, ColorFormat>>,
    main_depth: RefCell<
        gfx::handle::DepthStencilView<
            gfx_device_gl::Resources,
            (gfx::format::D24_S8, gfx::format::Unorm),
        >,
    >,

    last_gaze: RefCell<DeviceGaze>,
    active: RefCell<bool>,
    values: RefCell<ValueMap>,
}

impl Window {
    pub fn new(visible: bool, remote: Option<Remote>, values: ValueMap) -> Self {
        // Create a window and context.
        let gl_version = GlRequest::GlThenGles {
            opengles_version: (3, 2),
            opengl_version: (4, 1),
        };
        let events_loop = glutin::EventsLoop::new();
        let window_builder = glutin::WindowBuilder::new()
            .with_title("Visual System Simulator")
            .with_min_dimensions(LogicalSize::new(320.0, 200.0))
            .with_visibility(visible);
        let context_builder = glutin::ContextBuilder::new()
            .with_vsync(true)
            .with_gl(gl_version);
        let (windowed_context, mut device, mut factory, render_target, main_depth) =
            gfx_window_glutin::init::<ColorFormat, DepthFormat>(
                window_builder,
                context_builder,
                &events_loop,
            )
            .unwrap();

        // Create a command buffer.
        let encoder: gfx::Encoder<gfx_device_gl::Resources, gfx_device_gl::CommandBuffer> =
            factory.create_command_buffer().into();

        unsafe {
            device.with_gl(|gl| gl.Disable(gfx_gl::FRAMEBUFFER_SRGB));
        }

        Window {
            pipeline: Pipeline::new(),
            remote,
            windowed_context,
            events_loop: RefCell::new(events_loop),
            device: RefCell::new(device),
            factory: RefCell::new(factory),
            encoder: RefCell::new(encoder),
            render_target: RefCell::new(render_target),
            main_depth: RefCell::new(main_depth),
            last_gaze: RefCell::new(DeviceGaze { x: 0.5, y: 0.5 }),
            active: RefCell::new(false),
            values: RefCell::new(values),
        }
    }
}

impl Window {
    pub fn add_node(&mut self, node: Box<dyn Node>) {
        self.pipeline.add_node(node);
    }

    pub fn replace_node(&mut self, index: usize, node: Box<dyn Node>) {
        self.pipeline.replace_node(index, node);
    }

    pub fn nodes_len(&self) -> usize {
        self.pipeline.nodes_len()
    }

    pub fn update_nodes(&mut self) {
        self.pipeline.update_io(&self);
        self.pipeline.update_values(&self, &self.values.borrow());
    }

    pub fn set_values(&self, values: ValueMap) {
        self.values.replace(values);
        self.pipeline.update_values(&self, &self.values.borrow());
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

    fn override_gaze(gaze: DeviceGaze, values: &ValueMap) -> DeviceGaze {
        if let (Some(gaze_x), Some(gaze_y)) = (values.get("gaze_x"), values.get("gaze_y")) {
            DeviceGaze {
                x: gaze_x.as_f64().unwrap_or(0.0) as f32,
                y: gaze_y.as_f64().unwrap_or(0.0) as f32,
            }
        } else {
            gaze
        }
    }

    pub fn target(&self) -> DeviceTarget {
        self.render_target.borrow().clone()
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
                                DeviceGaze {
                                    x: position.x as f32 / window_size.width as f32,
                                    y: 1.0 - (position.y as f32 / window_size.height as f32),
                                },
                                &self.values.borrow(),
                            );
                        }
                    }
                    glutin::WindowEvent::CursorLeft { .. } => {
                        if *self.active.borrow() {
                            deferred_gaze = Self::override_gaze(
                                DeviceGaze { x: 0.5, y: 0.5 },
                                &self.values.borrow(),
                            );
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
            self.pipeline.update_io(&self);
            self.pipeline.update_values(&self, &self.values.borrow());
        }

        // Update input.
        self.pipeline.input(&deferred_gaze);
        *self.last_gaze.borrow_mut() = deferred_gaze;

        self.encoder
            .borrow_mut()
            .clear(&self.render_target.borrow(), [68.0 / 255.0; 4]);
        self.pipeline.render(&self);

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
