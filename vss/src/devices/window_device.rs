use std::cell::RefCell;

use gfx;
use gfx_device_gl;
use gfx_window_glutin;
use glutin;
use glutin::dpi::*;
use glutin::GlRequest;

use super::*;
use crate::config::*;

// A buffer representing color information.
pub struct RGBBuffer {
    pub pixels_rgb: Box<[u8]>,
    pub width: usize,
    pub height: usize,
}

pub type DepthFormat = gfx::format::DepthStencil;

/// A device for window and context creation.
pub struct WindowDevice {
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

    active: RefCell<bool>,
    gaze: RefCell<DeviceGaze>,
    fallback_gaze: DeviceGaze,
}

impl WindowDevice {
    pub fn new(config: &Config) -> Self {
        let events_loop = glutin::EventsLoop::new();

        let gl_version = GlRequest::GlThenGles {
            opengles_version: (3, 2),
            opengl_version: (4, 1),
        };

        let window_builder = glutin::WindowBuilder::new()
            .with_title(format!("Visual System Simulator - {}", config.input))
            .with_min_dimensions(LogicalSize::new(320.0, 200.0));

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

        // create our command buffer
        let encoder: gfx::Encoder<gfx_device_gl::Resources, gfx_device_gl::CommandBuffer> =
            factory.create_command_buffer().into();

        unsafe {
            device.with_gl(|gl| gl.Disable(gfx_gl::FRAMEBUFFER_SRGB));
        }

        let window_size = windowed_context.window().get_inner_size().unwrap();
        let fallback_gaze = if let Some(ref gaze) = config.gaze {
            DeviceGaze {
                x: gaze.x,
                y: window_size.height as f32 - gaze.y,
            }
        } else {
            DeviceGaze {
                x: (window_size.width / 2.0) as f32,
                y: (window_size.height / 2.0) as f32,
            }
        };

        WindowDevice {
            windowed_context,
            events_loop: RefCell::new(events_loop),
            device: RefCell::new(device),
            factory: RefCell::new(factory),
            encoder: RefCell::new(encoder),
            render_target: RefCell::new(render_target),
            main_depth: RefCell::new(main_depth),
            active: RefCell::new(false),
            gaze: RefCell::new(DeviceGaze {
                x: fallback_gaze.x,
                y: fallback_gaze.y,
            }),
            fallback_gaze,
        }
    }

    pub fn download_rgb(&self) -> RGBBuffer {
        use gfx::format::Formatted;
        use gfx::memory::Typed;
        use gfx::traits::FactoryExt;
        use gfx::Factory;
        use std::ops::DerefMut;

        let factory = &mut self.factory().borrow_mut();
        let encoder = &mut self.encoder().borrow_mut();
        let target = &mut self.target().borrow_mut();
        let (width, height, _, _) = target.get_dimensions();
        let width = width as usize;
        let height = height as usize;

        // Schedule download.
        let download = factory
            .create_download_buffer::<[u8; 4]>(width * height)
            .unwrap();
        encoder
            .copy_texture_to_buffer_raw(
                target.raw().get_texture(),
                None,
                gfx::texture::RawImageInfo {
                    xoffset: 0,
                    yoffset: 0,
                    zoffset: 0,
                    width: width as u16,
                    height: height as u16,
                    depth: 0,
                    format: ColorFormat::get_format(),
                    mipmap: 0,
                },
                download.raw(),
                0,
            )
            .unwrap();

        // Flush before reading the buffers to prevent panics.
        let device = &mut self.device.borrow_mut();
        encoder.flush(device.deref_mut());

        // Copy to buffers.
        let mut pixels_rgb = Vec::with_capacity(width * height * 3);
        let reader = factory.read_mapping(&download).unwrap();
        for row in reader.chunks(width as usize).rev() {
            for pixel in row.iter() {
                pixels_rgb.push(pixel[0]);
                pixels_rgb.push(pixel[1]);
                pixels_rgb.push(pixel[2]);
            }
        }

        RGBBuffer {
            pixels_rgb: pixels_rgb.into_boxed_slice(),
            width,
            height,
        }
    }
}

impl Device for WindowDevice {
    fn factory(&self) -> &RefCell<DeviceFactory> {
        &self.factory
    }

    fn encoder(&self) -> &RefCell<DeviceEncoder> {
        &self.encoder
    }

    fn gaze(&self) -> DeviceGaze {
        self.gaze.borrow().clone()
    }

    fn source(&self) -> &RefCell<DeviceSource> {
        panic!("Function not meant to be called - a window has no source");
    }

    fn target(&self) -> &RefCell<DeviceTarget> {
        &self.render_target
    }

    fn begin_frame(&self) {}

    fn end_frame(&self, done: &mut bool) {
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
                    | glutin::WindowEvent::Destroyed => *done = true,
                    glutin::WindowEvent::Resized(size) => {
                        let windowed_context = &self.windowed_context;
                        let mut rt = self.render_target.borrow_mut();
                        let mut md = self.main_depth.borrow_mut();
                        let dpi_factor = windowed_context.window().get_hidpi_factor();
                        windowed_context.resize(size.to_physical(dpi_factor));
                        gfx_window_glutin::update_views(&windowed_context, &mut rt, &mut md);
                    }
                    glutin::WindowEvent::CursorMoved { position, .. } => {
                        if *self.active.borrow() {
                            let window_size =
                                &self.windowed_context.window().get_inner_size().unwrap();
                            self.gaze.replace(DeviceGaze {
                                x: position.x as f32,
                                y: (window_size.height - position.y) as f32,
                            });
                        }
                    }
                    glutin::WindowEvent::CursorEntered { .. } => {
                        self.active.replace(true);
                    }
                    glutin::WindowEvent::CursorLeft { .. } => {
                        self.active.replace(false);
                        self.gaze.replace(DeviceGaze {
                            x: self.fallback_gaze.x,
                            y: self.fallback_gaze.y,
                        });
                    }
                    _ => (),
                }
            }
        });

        {
            use gfx::Device;
            use std::ops::DerefMut;
            let mut device = self.device.borrow_mut();
            self.encoder.borrow_mut().flush(device.deref_mut());
            self.windowed_context.swap_buffers().unwrap();
            device.cleanup();
        }
    }
}
