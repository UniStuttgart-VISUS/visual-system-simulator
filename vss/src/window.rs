use crate::*;
use std::cell::RefCell;
use cgmath::{Vector3};

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
    should_swap_buffers: RefCell<bool>,

    active: RefCell<bool>,
    values: Vec<RefCell<ValueMap>>,

    remote: Option<Remote>,
    flow: Vec<Flow>,
    vis_param: RefCell<VisualizationParameters>
}

impl Window {
    pub fn new(visible: bool, remote: Option<Remote>, values: Vec<RefCell<ValueMap>>, flow_count: usize) -> Self {
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

        let mut flow = Vec::new();
        flow.resize_with(flow_count, Flow::new);

        Window {
            flow,
            remote,
            windowed_context,
            events_loop: RefCell::new(events_loop),
            device: RefCell::new(device),
            factory: RefCell::new(factory),
            encoder: RefCell::new(encoder),
            render_target: RefCell::new(render_target),
            main_depth: RefCell::new(main_depth),
            should_swap_buffers: RefCell::new(true),
            active: RefCell::new(false),
            values: values,
            vis_param: RefCell::new(VisualizationParameters::default())
        }
    }
}

impl Window {
    pub fn add_node(&mut self, node: Box<dyn Node>, flow_index: usize) {
        self.flow[flow_index].add_node(node);
    }

    pub fn replace_node(&mut self, index: usize, node: Box<dyn Node>, flow_index: usize) {
        self.flow[flow_index].replace_node(index, node);
    }

    pub fn nodes_len(&self) -> usize {//TODO: return vector of lengths
        self.flow[0].nodes_len()
    }

    pub fn update_last_node(&mut self) {
        self.flow.iter().for_each(|f| f.update_last_slot(&self));
    }

    pub fn update_nodes(&mut self) {
        for (i, f) in self.flow.iter().enumerate(){
            f.negociate_slots(&self);
            f.update_values(&self, &self.values[i].borrow());
        }
    }

    pub fn set_values(&self, values: ValueMap, flow_index: usize) {
        self.values[flow_index].replace(values);
        self.flow[flow_index].update_values(&self, &self.values[flow_index].borrow());
    }
    
    pub fn set_value(&self, key: String, value: Value, flow_index: usize) {
        self.values[flow_index].borrow_mut().insert(key, value);
        self.flow[flow_index].update_values(&self, &self.values[flow_index].borrow());
    }
    
    pub fn set_head(&self, new_head: Head, flow_index: usize) {
        self.flow[flow_index].last_head.replace(new_head);
    }

    pub fn set_gaze(&self, new_gaze: Gaze, flow_index: usize) {
        self.flow[flow_index].last_gaze.replace(new_gaze);
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
                direction: Vector3::new(0.0, 0.0, 0.0),
            }
        } else {
            gaze
        }
    }

    pub fn target(&self) -> gfx::handle::RenderTargetView<gfx_device_gl::Resources, ColorFormat> {
        self.render_target.borrow().clone()
    }

    pub fn replace_targets(&self, target_color: RenderTargetColor, target_depth: RenderTargetDepth, should_swap_buffers: bool) {
        self.render_target.replace(target_color);
        self.main_depth.replace(target_depth);
        self.should_swap_buffers.replace(should_swap_buffers);
    }

    pub fn poll_events(&self) -> bool {
        let mut done = false;
        let mut deferred_size = None;
        let mut deferred_gaze =
            Self::override_gaze(self.flow[0].last_gaze.borrow().clone(), &self.values[0].borrow());//TODO: FIX THIS (don't just use the first flow)

        // Poll for window events.
        self.events_loop.borrow_mut().poll_events(|event| {
            if let glutin::Event::WindowEvent { event, .. } = event {
                match event {
                    glutin::WindowEvent::KeyboardInput {
                        input:
                            glutin::KeyboardInput {
                                state: glutin::ElementState::Pressed,
                                virtual_keycode: Some(glutin::VirtualKeyCode::H),
                                ..
                            },
                        ..
                    } => {
                        let mut vp = self.vis_param.borrow_mut();
                        vp.test_depth_max+=100.0;
                        println!("Depth min,max [{},{}]",
                            vp.test_depth_min,
                            vp.test_depth_max
                        );
                    },
                    glutin::WindowEvent::KeyboardInput {
                        input:
                            glutin::KeyboardInput {
                                state: glutin::ElementState::Pressed,
                                virtual_keycode: Some(glutin::VirtualKeyCode::L),
                                ..
                            },
                        ..
                    } => {
                        let mut vp = self.vis_param.borrow_mut();
                        vp.test_depth_max-=100.0;
                        println!("Depth min,max [{},{}]",
                            vp.test_depth_min,
                            vp.test_depth_max
                        );
                    },
                    glutin::WindowEvent::KeyboardInput {
                        input:
                            glutin::KeyboardInput {
                                state: glutin::ElementState::Pressed,
                                virtual_keycode: Some(glutin::VirtualKeyCode::J),
                                ..
                            },
                        ..
                    } => {
                        let mut vp = self.vis_param.borrow_mut();
                        vp.test_depth_min+=10.0;
                        println!("Depth min,max [{},{}]",
                            vp.test_depth_min,
                            vp.test_depth_max
                        );
                    },
                    glutin::WindowEvent::KeyboardInput {
                        input:
                            glutin::KeyboardInput {
                                state: glutin::ElementState::Pressed,
                                virtual_keycode: Some(glutin::VirtualKeyCode::K),
                                ..
                            },
                        ..
                    } => {
                        let mut vp = self.vis_param.borrow_mut();
                        vp.test_depth_min-=10.0;
                        println!("Depth min,max [{},{}]",
                            vp.test_depth_min,
                            vp.test_depth_max
                        );
                    },
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
                                virtual_keycode: Some(glutin::VirtualKeyCode::Key2),
                                ..
                            },
                        ..
                    } => {
                        self.vis_param.borrow_mut().vis_type=VisualizationType::ColorChange;
                    },
                    glutin::WindowEvent::KeyboardInput {
                        input:
                            glutin::KeyboardInput {
                                virtual_keycode: Some(glutin::VirtualKeyCode::Key3),
                                ..
                            },
                        ..
                    } => {
                        self.vis_param.borrow_mut().vis_type=VisualizationType::ColorUncertainty;
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
                                    direction: Vector3::new(0.0, 0.0, 0.0),
                                },
                                &self.values[0].borrow(),//TODO: FIX THIS (don't just use the first flow)
                            );
                            let last_head = &mut self.flow[0].last_head.borrow_mut();
                            last_head.yaw = position.x as f32 / window_size.width as f32
                                * std::f32::consts::PI
                                * 2.0
                                - 0.5;
                            last_head.pitch = position.y as f32 / window_size.height as f32
                                * std::f32::consts::PI
                                - 0.5;//50 mm lens
                        }
                    }
                    glutin::WindowEvent::CursorLeft { .. } => {
                        if *self.active.borrow() {
                            deferred_gaze =
                                Self::override_gaze(Gaze { x: 0.5, y: 0.5, direction: Vector3::new(0.0, 0.0, 0.0)}, &self.values[0].borrow());//TODO: FIX THIS (don't just use the first flow)
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
            for (i, f) in self.flow.iter().enumerate(){
                f.negociate_slots(&self);
                f.update_values(&self, &self.values[i].borrow());
            }
        }

        // Update input.
        for flow_index in 0 .. self.flow.len() {
            //self.flow[flow_index].input(&self.last_head.borrow(), &deferred_gaze, &self.vis_param.borrow(), flow_index);
            self.flow[flow_index].input(&self.vis_param.borrow());
        }
        //*self.flow[0].last_gaze.borrow_mut() = deferred_gaze; //TODO fix this for window input

        self.encoder
            .borrow_mut()
            .clear(&self.render_target.borrow(), [68.0 / 255.0; 4]);
        self.flow.iter().for_each(|f| f.render(&self));

        use gfx::Device;
        self.flush(&mut self.encoder().borrow_mut());
        self.device.borrow_mut().cleanup();

        if *self.should_swap_buffers.borrow(){
            self.windowed_context.swap_buffers().unwrap();
        }

        if let Some(remote) = &self.remote {
            remote.send_frame();
        }

        return done;
    }
}
