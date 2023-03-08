use crate::{*, Texture};
use std::rc::Rc;
use std::{cell::RefCell, borrow::BorrowMut};
use std::time::Instant;
use std::iter;
use std::num::NonZeroU32;
use cgmath::{Matrix4, Vector4, SquareMatrix};
use wgpu::*;

/// A factory to create device objects.
//TODO-WGPU pub type DeviceFactory = gfx_device_gl::Factory;

/// An encoder to manipulate a device command queue.
//TODO-WGPU pub type DeviceEncoder = gfx::Encoder<gfx_device_gl::Resources, gfx_device_gl::CommandBuffer>;

/// Render Target Types of this Window.
//TODO-WGPU pub type RenderTargetColor = gfx::handle::RenderTargetView<gfx_device_gl::Resources, ColorFormat>;
//TODO-WGPU pub type RenderTargetDepthFormat = (gfx::format::D24_S8, gfx::format::Unorm);
//TODO-WGPU pub type RenderTargetDepth = gfx::handle::DepthStencilView<gfx_device_gl::Resources, RenderTargetDepthFormat>;

/// Represents a window along with its associated rendering context and [Flow].
pub struct Surface {
    pub surface_size: [u32;2],
    pub surface: wgpu::Surface,
    surface_config: RefCell<wgpu::SurfaceConfiguration>,
    pub device: RefCell<wgpu::Device>,
    pub queue: RefCell<wgpu::Queue>,
    //TODO-WGPU factory: RefCell<DeviceFactory>,
    //TODO-WGPU encoder: RefCell<CommandEncoder>,

    //TODO-remove render_target: RefCell<TextureView>, //Rgba8Unorm
    //TODO-WGPU main_depth: RefCell<RenderTargetDepth>,
    should_swap_buffers: RefCell<bool>,
    pub cursor_pos: RefCell<[f64;2]>,
    pub override_view: RefCell<bool>,
    pub override_gaze: RefCell<bool>,

    pub active: RefCell<bool>,
    values: Vec<RefCell<ValueMap>>,

    pub remote: Option<Remote>,
    pub flow: Vec<Flow>,
    pub vis_param: RefCell<VisualizationParameters>,
    pub last_render_instant: RefCell<Instant>,
    pub forced_view: Option<(f32,f32)>
}

impl Surface {
    pub async fn new<W>(surface_size:[u32;2], window_handle: W, remote: Option<Remote>, values: Vec<RefCell<ValueMap>>, flow_count: usize) -> Self
    where  W: raw_window_handle::HasRawWindowHandle + raw_window_handle::HasRawDisplayHandle, {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::VULKAN,
            dx12_shader_compiler: wgpu::Dx12Compiler::Fxc,
        });
        let surface = unsafe { instance.create_surface(&window_handle) }.unwrap();
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

        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: COLOR_FORMAT,// surface.get_supported_formats(&adapter)[0],
            width: surface_size[0],
            height: surface_size[1],
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode: wgpu::CompositeAlphaMode::Auto,
            view_formats: vec![COLOR_FORMAT.add_srgb_suffix()],
        };
        surface.configure(&device, &surface_config);

        // Create a command buffer.
        //TODO-WGPU let encoder: CommandEncoder = factory.create_command_buffer().into();

        //TODO-WGPU maybe use "surface.get_supported_formats(&adapter)[0].describe().srgb" to filter;
        // unsafe {
        //     device.with_gl(|gl| gl.Disable(gfx_gl::FRAMEBUFFER_SRGB));
        //     
        // }

        let mut flow = Vec::new();
        flow.resize_with(flow_count, Flow::new);

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

        Surface {
           // wgpu_window,
         //   events_loop: RefCell::new(events_loop),
             surface_size,
            surface,
            surface_config: RefCell::new(surface_config),
            device: RefCell::new(device),
            queue: RefCell::new(queue),
            //TODO-WGPU factory: RefCell::new(factory),
            //TODO-WGPU encoder: RefCell::new(encoder),
            //TODO-WGPU main_depth: RefCell::new(main_depth),
            flow,
            remote,
            should_swap_buffers: RefCell::new(true),
            cursor_pos: RefCell::new([0.0, 0.0]),
            override_view,
            override_gaze: RefCell::new(false),
            active: RefCell::new(false),
            values: values,
            vis_param,
            last_render_instant: RefCell::new(Instant::now()),
            forced_view,
        }
    }

    pub fn add_node(&mut self, node: Box<dyn Node>, flow_index: usize) {
        self.flow[flow_index].add_node(node);
    }

    pub fn replace_node(&mut self, index: usize, node: Box<dyn Node>, flow_index: usize) {
        self.flow[flow_index].replace_node(index, node);
    }

    pub fn resize(&mut self, new_size: [u32;2]) {
        if new_size[0] > 0 && new_size[1] > 0 {
            self.surface_size = [new_size[0], new_size[1]];
            self.surface_config.borrow_mut().width = new_size[0];
            self.surface_config.borrow_mut().height = new_size[1];
            self.surface.configure(&self.device.borrow_mut(), &self.surface_config.borrow());
        }
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

    // pub fn update_last_node(&mut self) {
    //     self.flow.iter().for_each(|f| f.update_last_slot(&self));
    // }

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
    
    // pub fn set_perspective(&self, new_perspective: EyePerspective, flow_index: usize) {
    //     self.flow[flow_index].last_perspective.replace(new_perspective);
    // }

    // pub fn factory(&self) -> &RefCell<DeviceFactory> {
    //     &self.factory
    // }

    // pub fn encoder(&self) -> &RefCell<DeviceEncoder> {
    //     &self.encoder
    // }

    pub fn device(&self) -> & RefCell<wgpu::Device> {
        &self.device
    }

    pub fn queue(&self) -> & RefCell<wgpu::Queue> {
        &self.queue
    }

    pub fn surface_config(&self) -> & RefCell<wgpu::SurfaceConfiguration> {
        &self.surface_config
    }

    // pub fn flush(&self, encoder: &mut DeviceEncoder) {
    //     use std::ops::DerefMut;
    //     let mut device = self.device.borrow_mut();
    //     encoder.flush(device.deref_mut());
    // }

    // pub fn target(&self) -> & RefCell<TextureView> {
    //     &self.render_target
    //     //self.render_target.borrow().clone()
    // }

    // pub fn replace_targets(&self, target_color: RenderTargetColor, target_depth: RenderTargetDepth, should_swap_buffers: bool) {
    //     self.render_target.replace(target_color);
    //     self.main_depth.replace(target_depth);
    //     self.should_swap_buffers.replace(should_swap_buffers);
    // }

}
