use crate::*;
use std::cell::RefCell;
use std::time::Instant;
use cgmath::{Matrix4, Vector4, SquareMatrix};
use wgpu::*;
use winit::{
    event::*,
    dpi::*,
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};
// use glutin::{ElementState, MouseButton, dpi::{LogicalPosition}};

/// A factory to create device objects.
//TODO-WGPU pub type DeviceFactory = gfx_device_gl::Factory;

/// An encoder to manipulate a device command queue.
//TODO-WGPU pub type DeviceEncoder = gfx::Encoder<gfx_device_gl::Resources, gfx_device_gl::CommandBuffer>;

/// Render Target Types of this Window.
//TODO-WGPU pub type RenderTargetColor = gfx::handle::RenderTargetView<gfx_device_gl::Resources, ColorFormat>;
//TODO-WGPU pub type RenderTargetDepthFormat = (gfx::format::D24_S8, gfx::format::Unorm);
//TODO-WGPU pub type RenderTargetDepth = gfx::handle::DepthStencilView<gfx_device_gl::Resources, RenderTargetDepthFormat>;

/// Represents a window along with its associated rendering context and [Flow].
pub struct Window {
    windowed_context: winit::window::Window,
    events_loop: RefCell<EventLoop<()>>,
    device: RefCell<Device>,
    queue: RefCell<Queue>,
    //TODO-WGPU factory: RefCell<DeviceFactory>,
    //TODO-WGPU encoder: RefCell<CommandEncoder>,

    //TODO-WGPU render_target: RefCell<RenderTargetColor>,
    //TODO-WGPU main_depth: RefCell<RenderTargetDepth>,
    should_swap_buffers: RefCell<bool>,
    cursor_pos: RefCell<LogicalPosition<f32>>,
    override_view: RefCell<bool>,
    override_gaze: RefCell<bool>,

    active: RefCell<bool>,
    values: Vec<RefCell<ValueMap>>,

    remote: Option<Remote>,
    //TODO-WGPU flow: Vec<Flow>,
    vis_param: RefCell<VisualizationParameters>,
    last_render_instant: RefCell<Instant>,
    forced_view: Option<(f32,f32)>
}

impl Window {
    pub async fn new(visible: bool, remote: Option<Remote>, values: Vec<RefCell<ValueMap>>, flow_count: usize) -> Self {
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
        // let (windowed_context, mut device, mut factory, render_target, main_depth) =
        //     gfx_window_glutin::init::<
        //         (gfx::format::R8_G8_B8_A8, gfx::format::Unorm),
        //         gfx::format::DepthStencil,
        //     >(window_builder, context_builder, &events_loop)
        //     .unwrap();
        
        let windowed_context = window_builder.build(&events_loop).unwrap();

        windowed_context.set_cursor_visible(true);

        let instance = wgpu::Instance::new(wgpu::Backends::VULKAN);
        let surface = unsafe { instance.create_surface(&windowed_context) };
        let adapter = instance.request_adapter(
            &wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            },
        ).await.unwrap();


        let (device, queue) = adapter.request_device(
            &wgpu::DeviceDescriptor {
                features: wgpu::Features::empty(),
                // WebGL doesn't support all of wgpu's features, so if
                // we're building for the web we'll have to disable some.
                // TODO-WGPU will look into this at a later point
                // limits: if cfg!(target_arch = "wasm32") {
                //     wgpu::Limits::downlevel_webgl2_defaults()
                // } else {
                //     wgpu::Limits::default()
                // },
                limits: wgpu::Limits::default(),
                label: None,
            },
            None,
        ).await.unwrap();

        // Create a command buffer.
        //TODO-WGPU let encoder: CommandEncoder = factory.create_command_buffer().into();

        unsafe {
            //TODO-WGPU device.with_gl(|gl| gl.Disable(gfx_gl::FRAMEBUFFER_SRGB));
        }

        //TODO-WGPU let mut flow = Vec::new();
        //TODO-WGPU flow.resize_with(flow_count, Flow::new);

        //TODO set perspective from values here ?

        let mut vis_param = VisualizationParameters::default();
        if let Some(Value::Number(file_base_image)) = values[0].borrow().get("file_base_image") {
            vis_param.vis_type.base_image = match *file_base_image as i32{
                0 => BaseImage::Output,
                1 => BaseImage::Original,
                2 => BaseImage::Ganglion,
                _ => panic!("No BaseImage of {} found", file_base_image)
            };
        }
        if let Some(Value::Number(file_mix_type)) = values[0].borrow().get("file_mix_type") {
            vis_param.vis_type.mix_type = match *file_mix_type as i32{
                0 => MixType::BaseImageOnly,
                1 => MixType::ColorMapOnly,
                2 => MixType::OverlayThreshold,
                _ => panic!("No MixType of {} found", file_mix_type)
            };
        }
        if let Some(Value::Number(file_color_map_type)) = values[0].borrow().get("file_cm_type") {
            vis_param.vis_type.color_map_type = match *file_color_map_type as i32{
                0 => ColorMapType::Viridis,
                1 => ColorMapType::Turbo,
                2 => ColorMapType::Grayscale,
                _ => panic!("No ColorMapType of {} found", file_color_map_type)
            };
        }
        if let Some(Value::Number(combination_function)) = values[0].borrow().get("file_cf") {
            vis_param.vis_type.combination_function = match *combination_function as i32{
                0 => CombinationFunction::AbsoluteErrorRGBVectorLength,
                1 => CombinationFunction::AbsoluteErrorXYVectorLength,
                2 => CombinationFunction::AbsoluteErrorRGBXYVectorLength,
                3 => CombinationFunction::UncertaintyRGBVectorLength,
                4 => CombinationFunction::UncertaintyXYVectorLength,
                5 => CombinationFunction::UncertaintyRGBXYVectorLength,
                6 => CombinationFunction::UncertaintyGenVar,
                _ => panic!("No CombinationFunction of {} found", combination_function)
            };
        }
        if let Some(Value::Number(cm_scale)) = values[0].borrow().get("cm_scale") {
            vis_param.heat_scale = *cm_scale as f32;
        }
        if let Some(Value::Number(measure_variance)) = values[0].borrow().get("measure_variance") {
            vis_param.measure_variance = *measure_variance as u32;
        }
        if let Some(Value::Number(variance_metric)) = values[0].borrow().get("variance_metric") {
            vis_param.variance_metric = *variance_metric as u32;
        }
        if let Some(Value::Number(variance_color_space)) = values[0].borrow().get("variance_color_space") {
            vis_param.variance_color_space = *variance_color_space as u32;
        }

        let mut override_view = false;

        let mut forced_view = None;
        if let (Some(Value::Number(view_x)), Some(Value::Number(view_y)) ) = (values[0].borrow().get("view_x"),(values[0].borrow().get("view_y"))) {
            forced_view = Some((*view_x as f32,*view_y as f32));
            override_view = true;
        }
        let override_view =  RefCell::new(override_view);

        let vis_param = RefCell::new(vis_param);

        Window {
            //TODO-WGPU flow,
            remote,
            windowed_context,
            events_loop: RefCell::new(events_loop),
            device: RefCell::new(device),
            queue: RefCell::new(queue),
            //TODO-WGPU factory: RefCell::new(factory),
            //TODO-WGPU encoder: RefCell::new(encoder),
            //TODO-WGPU render_target: RefCell::new(render_target),
            //TODO-WGPU main_depth: RefCell::new(main_depth),
            should_swap_buffers: RefCell::new(true),
            cursor_pos: RefCell::new(LogicalPosition{x:0.0, y:0.0}),
            override_view,
            override_gaze: RefCell::new(false),
            active: RefCell::new(false),
            values: values,
            vis_param,
            last_render_instant: RefCell::new(Instant::now()),
            forced_view,
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

    pub fn delta_t(&self)  -> f32{
        if self.vis_param.borrow().bees_flying {
            return self.last_render_instant.borrow().elapsed().as_micros() as f32;
        }
        return 0.0;
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

    pub fn device(&self) -> & RefCell<gfx_device_gl::Device> {
        &self.device
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
            if let Event::WindowEvent { event, .. } = event {
                match event {
                    WindowEvent::KeyboardInput {
                        input:
                            KeyboardInput {state,
                                virtual_keycode: Some(VirtualKeyCode::LShift),
                                ..
                            },
                        ..
                    } => {
                        match state{
                            ElementState::Pressed => {
                                let mut vp = self.vis_param.borrow_mut();
                                vp.edit_eye_position = 1;
                            },
                            ElementState::Released => {
                                let mut vp = self.vis_param.borrow_mut();
                                vp.edit_eye_position = 0;
                            },
                        }
                    }, 
                    WindowEvent::KeyboardInput {
                        input:
                            KeyboardInput {
                                state: ElementState::Pressed,
                                virtual_keycode: Some(VirtualKeyCode::P),
                                ..
                            },
                        ..
                    } => {
                        let mut vp = self.vis_param.borrow_mut();
                        vp.bees_flying = !vp.bees_flying;
                    }, 
                    WindowEvent::KeyboardInput {
                        input:
                            KeyboardInput {
                                state: ElementState::Pressed,
                                virtual_keycode: Some(VirtualKeyCode::B),
                                ..
                            },
                        ..
                    } => {
                        let mut vp = self.vis_param.borrow_mut();
                        vp.bees_visible = !vp.bees_visible;
                    }, 
                    WindowEvent::KeyboardInput {
                        input:
                            KeyboardInput {
                                state: ElementState::Pressed,
                                virtual_keycode: Some(VirtualKeyCode::R),
                                ..
                            },
                        ..
                    } => {
                        let mut vp = self.vis_param.borrow_mut();
                        if vp.edit_eye_position > 0 {
                            vp.eye_position = (0.0, 0.0);
                        }
                    }, 
                    WindowEvent::KeyboardInput {
                        input:
                            KeyboardInput {
                                state: ElementState::Pressed,
                                virtual_keycode: Some(VirtualKeyCode::Space),
                                ..
                            },
                        ..
                    } => {
                        let mut vp = self.vis_param.borrow_mut();
                        // println!("Space: eye was {}",(vp.eye_idx as u32));
                        vp.eye_idx = (vp.eye_idx+1)%2
                    },                
                    WindowEvent::KeyboardInput {
                        input:
                            KeyboardInput {
                                state: ElementState::Pressed,
                                virtual_keycode: Some(VirtualKeyCode::H),
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
                    WindowEvent::KeyboardInput {
                        input:
                            KeyboardInput {
                                state: ElementState::Pressed,
                                virtual_keycode: Some(VirtualKeyCode::L),
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
                    WindowEvent::KeyboardInput {
                        input:
                            KeyboardInput {
                                state: ElementState::Pressed,
                                virtual_keycode: Some(VirtualKeyCode::J),
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
                    WindowEvent::KeyboardInput {
                        input:
                            KeyboardInput {
                                state: ElementState::Pressed,
                                virtual_keycode: Some(VirtualKeyCode::K),
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
                    WindowEvent::KeyboardInput {
                        input:
                            KeyboardInput {
                                state: ElementState::Pressed,
                                virtual_keycode: Some(VirtualKeyCode::A),
                                ..
                            },
                        ..
                    } => {
                        self.vis_param.borrow_mut().dir_calc_scale-=5.0;
                    },
                    WindowEvent::KeyboardInput {
                        input:
                            KeyboardInput {
                                state: ElementState::Pressed,
                                virtual_keycode: Some(VirtualKeyCode::D),
                                ..
                            },
                        ..
                    } => {
                        self.vis_param.borrow_mut().dir_calc_scale+=5.0;
                    },
                    WindowEvent::KeyboardInput {
                        input:
                            KeyboardInput {
                                state: ElementState::Pressed,
                                virtual_keycode: Some(VirtualKeyCode::W),
                                ..
                            },
                        ..
                    } => {
                        self.vis_param.borrow_mut().heat_scale+=0.5;
                    },
                    WindowEvent::KeyboardInput {
                        input:
                            KeyboardInput {
                                state: ElementState::Pressed,
                                virtual_keycode: Some(VirtualKeyCode::S),
                                ..
                            },
                        ..
                    } => {
                        self.vis_param.borrow_mut().heat_scale-=0.5;
                    },
                    WindowEvent::KeyboardInput {
                        input:
                            KeyboardInput {
                                state: ElementState::Pressed,
                                virtual_keycode: Some(VirtualKeyCode::Q),
                                ..
                            },
                        ..
                    } => {
                        self.vis_param.borrow_mut().astigmatism_strength-=0.5;
                    },
                    WindowEvent::KeyboardInput {
                        input:
                            KeyboardInput {
                                state: ElementState::Pressed,
                                virtual_keycode: Some(VirtualKeyCode::E),
                                ..
                            },
                        ..
                    } => {
                        self.vis_param.borrow_mut().astigmatism_strength+=0.5;
                    },
                    WindowEvent::KeyboardInput {
                        input:
                            KeyboardInput {
                                virtual_keycode: Some(VirtualKeyCode::C),
                                state: ElementState::Pressed,
                                ..
                            },
                        ..
                    } => {
                        let mut vp = self.vis_param.borrow_mut();
                        match vp.vis_type.color_map_type {
                            ColorMapType::Viridis => (*vp).vis_type.color_map_type = ColorMapType::Turbo,
                            ColorMapType::Turbo => (*vp).vis_type.color_map_type = ColorMapType::Grayscale,
                            ColorMapType::Grayscale  => (*vp).vis_type.color_map_type = ColorMapType::Viridis,
                        }
                    },
                    WindowEvent::KeyboardInput {
                        input:
                            KeyboardInput {
                                virtual_keycode: Some(VirtualKeyCode::Escape),
                                ..
                            },
                        ..
                    }
                    | WindowEvent::CloseRequested
                    | WindowEvent::Destroyed => done = true,
                    WindowEvent::Focused(active) => {
                        self.active.replace(active);
                    }
                    WindowEvent::Resized(size) => {
                        deferred_size = Some(size);
                    }
                    WindowEvent::CursorMoved { position, .. } => {
                        if *self.active.borrow() {
                            self.cursor_pos.replace(position);
                            let mut vp = self.vis_param.borrow_mut();
                            vp.mouse_input.position = (position.x as f32, position.y as f32);
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
                    WindowEvent::CursorLeft { .. } => {
                        if *self.active.borrow() {
                            self.override_view.replace(false);
                            self.override_gaze.replace(false);
                            //reset gaze ?
                        }
                    }
                    WindowEvent::MouseInput { state, button, .. } => {
                        if *self.active.borrow() {
                            let mut vp = self.vis_param.borrow_mut();
                            match button {
                                MouseButton::Left => {
                                    self.override_view.replace(state == ElementState::Pressed);
                                    vp.mouse_input.left_button = state == ElementState::Pressed;
                                }
                                MouseButton::Right => {
                                    self.override_gaze.replace(state == ElementState::Pressed);
                                    vp.mouse_input.right_button = state == ElementState::Pressed;
                                }
                                _ => {}
                            }
                        }
                    }
                    WindowEvent::KeyboardInput {
                        input:KeyboardInput {virtual_keycode, ..}, ..
                    } => {
                        let mut vp = self.vis_param.borrow_mut();
                        match virtual_keycode{
                            Some(VirtualKeyCode::O) => vp.vis_type.base_image=BaseImage::Output,
                            Some(VirtualKeyCode::I) => vp.vis_type.base_image=BaseImage::Original,
                            Some(VirtualKeyCode::G) => vp.vis_type.base_image=BaseImage::Ganglion,
                            Some(VirtualKeyCode::V) => vp.vis_type.base_image=BaseImage::Variance,

                            Some(VirtualKeyCode::Key1) => vp.vis_type.mix_type=MixType::BaseImageOnly,
                            Some(VirtualKeyCode::Key2) => vp.vis_type.mix_type=MixType::ColorMapOnly,
                            Some(VirtualKeyCode::Key3) => vp.vis_type.mix_type=MixType::OverlayThreshold,

                            Some(VirtualKeyCode::Key4) => vp.vis_type.combination_function=CombinationFunction::AbsoluteErrorRGBVectorLength,
                            Some(VirtualKeyCode::Key5) => vp.vis_type.combination_function=CombinationFunction::AbsoluteErrorXYVectorLength,
                            Some(VirtualKeyCode::Key6) => vp.vis_type.combination_function=CombinationFunction::AbsoluteErrorRGBXYVectorLength,
                            Some(VirtualKeyCode::Key7) => vp.vis_type.combination_function=CombinationFunction::UncertaintyRGBVectorLength,
                            Some(VirtualKeyCode::Key8) => vp.vis_type.combination_function=CombinationFunction::UncertaintyXYVectorLength,
                            Some(VirtualKeyCode::Key9) => vp.vis_type.combination_function=CombinationFunction::UncertaintyRGBXYVectorLength,
                            Some(VirtualKeyCode::Key0) => vp.vis_type.combination_function=CombinationFunction::UncertaintyGenVar,
                            _ => {}
                        }
                    }
                    _ => (),
                }
            }
        });

        if let Some(_) = self.forced_view {
            // Update pipline IO.
            let _dpi_factor = self.windowed_context.window().get_hidpi_factor();
            let size = glutin::dpi::PhysicalSize {
                width: 1920.0,
                height: 1080.0,
            };
            //self.windowed_context.resize(size);
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
                //println!("{} {}",cursor_pos.x as f32 ,cursor_pos.y as f32);
                let view_input = match self.forced_view {
                    Some(pos) =>{
                        pos
                    }
                    None =>{
                        (cursor_pos.x as f32 ,cursor_pos.y as f32)
                    }
                };

                self.vis_param.borrow_mut().highlight_position = (cursor_pos.x/window_size.width, cursor_pos.y/window_size.height);
                let yaw = view_input.0 as f32 / window_size.width as f32
                    * std::f32::consts::PI
                    * 2.0
                    - 0.5;
                let pitch = view_input.1 as f32 / window_size.height as f32
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
        //println!("Rendered with: {:?}", self.vis_param.borrow_mut());

        self.encoder
            .borrow_mut()
            .clear(&self.render_target.borrow(), [68.0 / 255.0; 4]);
        self.flow.iter().for_each(|f| f.render(&self));
        self.last_render_instant.replace(Instant::now());

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
