use crate::*;
use std::cell::RefCell;
use cgmath::{Matrix4, Vector4, SquareMatrix};
use glutin::{ElementState, MouseButton, dpi::{LogicalPosition}};

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
    cursor_pos: RefCell<LogicalPosition>,
    override_view: RefCell<bool>,
    override_gaze: RefCell<bool>,

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

        //TODO set perspective from values here ?

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
            cursor_pos: RefCell::new(LogicalPosition{x:0.0, y:0.0}),
            override_view: RefCell::new(false),
            override_gaze: RefCell::new(false),
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
    
    pub fn set_perspective(&self, new_perspective: EyePerspective, flow_index: usize) {
        self.flow[flow_index].last_perspective.replace(new_perspective);
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

        // Poll for window events.
        self.events_loop.borrow_mut().poll_events(|event| {
            if let glutin::Event::WindowEvent { event, .. } = event {
                match event {
                    glutin::WindowEvent::KeyboardInput {
                        input:
                            glutin::KeyboardInput {state,
                                virtual_keycode: Some(glutin::VirtualKeyCode::LShift),
                                ..
                            },
                        ..
                    } => {
                        match state{
                            glutin::ElementState::Pressed => {
                                let mut vp = self.vis_param.borrow_mut();
                                vp.edit_eye_position = 1;
                            },
                            glutin::ElementState::Released => {
                                let mut vp = self.vis_param.borrow_mut();
                                vp.edit_eye_position = 0;
                            },
                        }
                    }, 
                    glutin::WindowEvent::KeyboardInput {
                        input:
                            glutin::KeyboardInput {
                                state: glutin::ElementState::Pressed,
                                virtual_keycode: Some(glutin::VirtualKeyCode::R),
                                ..
                            },
                        ..
                    } => {
                        let mut vp = self.vis_param.borrow_mut();
                        if vp.edit_eye_position > 0 {
                            vp.eye_position = (0.0, 0.0);
                        }
                    }, 
                    glutin::WindowEvent::KeyboardInput {
                        input:
                            glutin::KeyboardInput {
                                state: glutin::ElementState::Pressed,
                                virtual_keycode: Some(glutin::VirtualKeyCode::Space),
                                ..
                            },
                        ..
                    } => {
                        let mut vp = self.vis_param.borrow_mut();
                        // println!("Space: eye was {}",(vp.eye_idx as u32));
                        vp.eye_idx = (vp.eye_idx+1)%2
                    },                
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
                                state: glutin::ElementState::Pressed,
                                virtual_keycode: Some(glutin::VirtualKeyCode::Q),
                                ..
                            },
                        ..
                    } => {
                        self.vis_param.borrow_mut().astigmatism_strength-=0.5;
                    },
                    glutin::WindowEvent::KeyboardInput {
                        input:
                            glutin::KeyboardInput {
                                state: glutin::ElementState::Pressed,
                                virtual_keycode: Some(glutin::VirtualKeyCode::E),
                                ..
                            },
                        ..
                    } => {
                        self.vis_param.borrow_mut().astigmatism_strength+=0.5;
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
                                virtual_keycode: Some(glutin::VirtualKeyCode::Key4),
                                ..
                            },
                        ..
                    } => {
                        self.vis_param.borrow_mut().vis_type=VisualizationType::Original;
                    },
                    glutin::WindowEvent::KeyboardInput {
                        input:
                            glutin::KeyboardInput {
                                virtual_keycode: Some(glutin::VirtualKeyCode::Key5),
                                ..
                            },
                        ..
                    } => {
                        self.vis_param.borrow_mut().vis_type=VisualizationType::OverlayOutput;
                    },
                    glutin::WindowEvent::KeyboardInput {
                        input:
                            glutin::KeyboardInput {
                                virtual_keycode: Some(glutin::VirtualKeyCode::Key6),
                                ..
                            },
                        ..
                    } => {
                        self.vis_param.borrow_mut().vis_type=VisualizationType::OverlayInput;
                    },
                    glutin::WindowEvent::KeyboardInput {
                        input:
                            glutin::KeyboardInput {
                                virtual_keycode: Some(glutin::VirtualKeyCode::Key7),
                                ..
                            },
                        ..
                    } => {
                        self.vis_param.borrow_mut().vis_type=VisualizationType::Ganglion;
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
                            self.cursor_pos.replace(position);
                            let mut vp = self.vis_param.borrow_mut();
                            match vp.edit_eye_position {
                                1 => {
                                    vp.previous_mouse_position = (position.x as f32 * 0.1, position.y as f32 * 0.1);
                                    vp.edit_eye_position = 2;
                                },
                                2 => {
                                    let (p_x,p_y) = vp.previous_mouse_position;
                                    let (c_x,c_y) = (position.x as f32 * 0.1, position.y as f32 * 0.1);
                                    vp.eye_position = (c_x - p_x, c_y - p_y);
                                    // println!("{:?}",vp.eye_position);
                                },
                                _ => {}
                            }
                        }
                    }
                    glutin::WindowEvent::CursorLeft { .. } => {
                        if *self.active.borrow() {
                            self.override_view.replace(false);
                            self.override_gaze.replace(false);
                            //reset gaze ?
                        }
                    }
                    glutin::WindowEvent::MouseInput { state, button, .. } => {
                        if *self.active.borrow() {
                            match button {
                                MouseButton::Left => {
                                    self.override_view.replace(state == ElementState::Pressed);
                                }
                                MouseButton::Right => {
                                    self.override_gaze.replace(state == ElementState::Pressed);
                                }
                                _ => {}
                            }
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
                f.last_perspective.borrow_mut().proj = cgmath::perspective(
                    cgmath::Deg(70.0), (size.width/size.height) as f32, 0.05, 1000.0);
            }
        }

        // Update input.
        for f in self.flow.iter(){
            if *self.override_view.borrow() || *self.override_gaze.borrow() {
                let window_size = &self.windowed_context.window().get_inner_size().unwrap();
                let cursor_pos = self.cursor_pos.borrow();
                let yaw = cursor_pos.x as f32 / window_size.width as f32
                    * std::f32::consts::PI
                    * 2.0
                    - 0.5;
                let pitch = cursor_pos.y as f32 / window_size.height as f32
                    * std::f32::consts::PI
                    - 0.5;//50 mm lens
                let view = Matrix4::from_angle_x(cgmath::Rad(pitch)) * Matrix4::from_angle_y(cgmath::Rad(yaw));

                let mut perspective = f.last_perspective.borrow_mut();

                if *self.override_view.borrow() {
                    if !*self.override_gaze.borrow(){
                        perspective.gaze = (view * perspective.view.invert().unwrap() * perspective.gaze.extend(1.0)).truncate();
                    }
                    perspective.view = view;
                }
                if *self.override_gaze.borrow() {
                    perspective.gaze = (perspective.view * view.invert().unwrap() * Vector4::unit_z()).truncate();
                }
            }
            f.input(&self.vis_param.borrow());
        }

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
