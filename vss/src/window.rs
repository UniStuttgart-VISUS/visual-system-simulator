use crate::*;
use std::{cell::RefCell};
use cgmath::{Matrix4, Vector4, SquareMatrix};
use winit::{
    event::*,
    dpi::*,
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder, platform::run_return::EventLoopExtRunReturn,
};

/// Represents a window along with its associated rendering context and [Flow].
pub struct Window {
    wgpu_window: winit::window::Window,
    events_loop: RefCell<EventLoop<()>>, 
    pub surface: surface::Surface,

    active: RefCell<bool>,
    cursor_pos: RefCell<(f32, f32)>,
    static_pos: Option<(f32,f32)>,

    override_gaze: RefCell<bool>,
    override_view: RefCell<bool>,
}

impl Window {
    pub async fn new(visible: bool, flow_count: usize, remote: Option<Remote>, static_pos: Option<(f32,f32)>) -> Self {
      
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
 
        let surface = surface::Surface::new(
            [window_size.width, window_size.height],
             &wgpu_window, flow_count, remote)
            .await;
    
        Window {
            wgpu_window,
            events_loop: RefCell::new(events_loop),
            surface,
            active: RefCell::new(false),
            cursor_pos: RefCell::new((0.0, 0.0)),
            static_pos,
            override_view: RefCell::new(static_pos.is_some()),
            override_gaze: RefCell::new(false),
        }
    }
 
    pub fn poll_events(&mut self) -> bool {
        let mut done = false;
        let mut deferred_size = None;
        let mut redraw_requested = true;

        // Poll for window events.
        // TODO-WGPU use .run() instead of .run_return() as it is highly discouraged and incompatible with some platforms
        self.events_loop.borrow_mut().run_return(|event, _, control_flow| {
            match event {
                Event::WindowEvent { window_id, ref event } if window_id == self.wgpu_window.id() =>{
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
                                    let mut vp = self.surface.vis_param.borrow_mut();
                                    vp.edit_eye_position = 1;
                                },
                                ElementState::Released => {
                                    let mut vp = self.surface.vis_param.borrow_mut();
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
                            let mut vp = self.surface.vis_param.borrow_mut();
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
                            let mut vp = self.surface.vis_param.borrow_mut();
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
                            let mut vp = self.surface.vis_param.borrow_mut();
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
                            let mut vp = self.surface.vis_param.borrow_mut();
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
                            let mut vp = self.surface.vis_param.borrow_mut();
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
                            let mut vp = self.surface.vis_param.borrow_mut();
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
                            let mut vp = self.surface.vis_param.borrow_mut();
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
                            let mut vp = self.surface.vis_param.borrow_mut();
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
                            self.surface.vis_param.borrow_mut().dir_calc_scale-=5.0;
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
                            self.surface.vis_param.borrow_mut().dir_calc_scale+=5.0;
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
                            self.surface.vis_param.borrow_mut().heat_scale+=0.5;
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
                            self.surface.vis_param.borrow_mut().heat_scale-=0.5;
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
                            self.surface.vis_param.borrow_mut().astigmatism_strength-=0.5;
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
                            self.surface.vis_param.borrow_mut().astigmatism_strength+=0.5;
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
                            let mut vp = self.surface.vis_param.borrow_mut();
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
                        | WindowEvent::Destroyed => {
                            done = true;
                        }
                        WindowEvent::Focused(active) => {
                            self.active.replace(*active);
                        }
                        WindowEvent::Resized(size) => {
                            deferred_size = Some(*size);
                        }
                        WindowEvent::CursorMoved { position, .. } => {
                            if *self.active.borrow() {
                                self.cursor_pos.replace((position.x as f32, position.y as f32));
                                let mut vp = self.surface.vis_param.borrow_mut();
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
                                let mut vp = self.surface.vis_param.borrow_mut();
                                match button {
                                    MouseButton::Left => {
                                        self.override_view.replace(*state == ElementState::Pressed);
                                        vp.mouse_input.left_button = *state == ElementState::Pressed;
                                    }
                                    MouseButton::Right => {
                                        self.override_gaze.replace(*state == ElementState::Pressed);
                                        vp.mouse_input.right_button = *state == ElementState::Pressed;
                                    }
                                    _ => {}
                                }
                            }
                        }
                        WindowEvent::KeyboardInput {
                            input:KeyboardInput {virtual_keycode, ..}, ..
                        } => {
                            let mut vp = self.surface.vis_param.borrow_mut();
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

        if let Some(_) = self.static_pos {
            // Update pipline IO.
            let new_size = PhysicalSize::new(1920 as u32, 1080 as u32);
            //self.wgpu_window.resize(size);
            self.surface.resize([new_size.width, new_size.height]);
            // TODO-WGPU
            // gfx_window_glutin::update_views(
            //     &self.wgpu_window,
            //     &mut self.render_target.borrow_mut(),
            //     &mut self.main_depth.borrow_mut(),
            // );
            // TODO-WGPU
            // for (i, f) in self.flow.iter().enumerate(){
            //     f.negociate_slots(&self);
            //     f.update_values(&self, &self.values[i].borrow());
            //     f.last_perspective.borrow_mut().proj = cgmath::perspective(
            //         cgmath::Deg(70.0), (size.width/size.height) as f32, 0.05, 1000.0);
            // }
        }

        if let Some(new_size) = deferred_size {
            // Update pipline IO.
            // let dpi_factor = self.wgpu_window.scale_factor();
            // let size = size.to_physical(dpi_factor);
            self.surface.resize([new_size.width, new_size.height]);
            // self.wgpu_window.resize(size);
            // TODO-WGPU 
            // gfx_window_glutin::update_views(
            //     &self.wgpu_window,
            //     &mut self.render_target.borrow_mut(),
            //     &mut self.main_depth.borrow_mut(),
            // );
            // TODO-WGPU
            // for (i, f) in self.flow.iter().enumerate(){
            //     f.negociate_slots(&self);
            //     f.update_values(&self, &self.values[i].borrow());
            //     f.last_perspective.borrow_mut().proj = cgmath::perspective(
            //         cgmath::Deg(70.0), (size.width/size.height) as f32, 0.05, 1000.0);
            // }
        }

        // Update input.
        for f in self.surface.flow.iter(){
            if *self.override_view.borrow() || *self.override_gaze.borrow() {
                let cursor_pos = self.cursor_pos.borrow().clone();
                //println!("{} {}",cursor_pos.x as f32 ,cursor_pos.y as f32);
                let view_pos =  self.static_pos.unwrap_or(cursor_pos);
                     

                self.surface.vis_param.borrow_mut().highlight_position = (cursor_pos.0/(self.surface.width() as f32), cursor_pos.1/(self.surface.height() as f32));
                let yaw = view_pos.0 / (self.surface.width() as f32)
                    * std::f32::consts::PI
                    * 2.0
                    - 0.5;
                let pitch = view_pos.1 / (self.surface.height() as f32)
                    * std::f32::consts::PI
                    - 0.5;//50 mm lens
                let view = Matrix4::from_angle_x(cgmath::Rad(pitch)) * Matrix4::from_angle_y(cgmath::Rad(yaw));

                let mut perspective = f.mut_perspective();

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
            f.input(&self.surface.vis_param.borrow());
        }
        // println!("Rendered with: {:?}", self.surface.vis_param.borrow_mut());

        if redraw_requested {
            self.surface.draw();
        }

        if let Some(remote) = &self.surface.remote {
            remote.send_frame();
        }

        return done;
    }
}
