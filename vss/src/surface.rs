use crate::*;
use std::iter;
use std::rc::Rc;
use std::{cell::RefCell};
use std::time::Instant;
use wgpu::{self, SurfaceTexture, SurfaceError};

/// Represents a rendering surface and its associated [Flow].
pub struct Surface {
    surface: wgpu::Surface,
    surface_size: [u32;2],
    surface_config: RefCell<wgpu::SurfaceConfiguration>,
    device: RefCell<wgpu::Device>,
    queue: RefCell<wgpu::Queue>,
 
    values: Vec<RefCell<ValueMap>>,

    pub remote: Option<Remote>,
    pub flow: Vec<Flow>,
    pub vis_param: RefCell<VisualizationParameters>,
    last_render_instant: RefCell<Instant>,
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

        // Query surface capablities, preferably with sRGB support.
        let swapchain_capabilities = surface.get_capabilities(&adapter);
        let mut view_formats = vec![];
        // #[cfg(target_os = "android")]
        // {
        //     let srgb_format = swapchain_capabilities.formats[0].add_srgb_suffix();
        //     if swapchain_capabilities.formats.contains(&srgb_format) {
        //         view_formats.push(srgb_format);
        //     }
        // }
 
        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            #[cfg(target_os = "android")]
            format: swapchain_capabilities.formats[0],
            #[cfg(target_os = "windows")]
            format: swapchain_capabilities.formats[0].remove_srgb_suffix(), // TODO find a better workaround for this (e.g. adjust output format of last node)
            width: surface_size[0],
            height: surface_size[1],
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode: swapchain_capabilities.alpha_modes[0],
            view_formats,
        };
        surface.configure(&device, &surface_config);

        // Create flows.
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

        let vis_param = RefCell::new(vis_param);

        Surface {
            surface,
            surface_size,
            surface_config: RefCell::new(surface_config),
            device: RefCell::new(device),
            queue: RefCell::new(queue),
            flow,
            remote,
            values,
            vis_param,
            last_render_instant: RefCell::new(Instant::now()),
        }
    }

    pub fn add_node(&mut self, node: Box<dyn Node>, flow_index: usize) {
        self.flow[flow_index].add_node(node);
    }

    pub fn replace_node(&mut self, index: usize, node: Box<dyn Node>, flow_index: usize) {
        self.flow[flow_index].replace_node(index, node);
    }

    pub fn resize(&mut self, new_size: [u32;2]) {
        assert!(new_size[0] > 0 && new_size[1] > 0, "Non-positive size");
        self.surface_size = [new_size[0], new_size[1]];
        self.surface_config.borrow_mut().width = new_size[0];
        self.surface_config.borrow_mut().height = new_size[1];
        self.surface.configure(&self.device.borrow_mut(), &self.surface_config.borrow());
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

    pub fn device(&self) -> & RefCell<wgpu::Device> {
        &self.device
    }

    pub fn queue(&self) -> & RefCell<wgpu::Queue> {
        &self.queue
    }

    pub fn surface_config(&self) -> & RefCell<wgpu::SurfaceConfiguration> {
        &self.surface_config
    }

    pub fn width(&self) -> u32 {
        return self.surface_size[0];
    }

    pub fn height(&self) -> u32 {
        return self.surface_size[1];
    }

    pub fn get_current_texture(&self) -> Result<SurfaceTexture, SurfaceError> {
        return self.surface.get_current_texture();
    }

    pub fn draw(&self) {
        let output = self.get_current_texture().unwrap();
    
        let view = output
        .texture
        .create_view(&wgpu::TextureViewDescriptor::default());

        let sampler = create_sampler_linear(&(self.device.borrow_mut()));

        let mut encoder = self
        .device
            .borrow_mut()
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        let render_texture = RenderTexture{
            texture: None,
            view: Rc::new(view),
            sampler: Rc::new(sampler),
            view_dimension: wgpu::TextureViewDimension::D2,
            width: self.width(),
            height: self.height(),
            label: "surface render texture".to_string(),
        };
    
        self.flow.iter().for_each(|f| f.render(&self, &mut encoder, &render_texture));

        self.queue.borrow_mut().submit(iter::once(encoder.finish()));
        output.present();
        self.flow.iter().for_each(|f| f.post_render(&self));
        self.last_render_instant.replace(Instant::now());
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

    // pub fn replace_targets(&self, target_color: RenderTargetColor, target_depth: RenderTargetDepth) {
    //     self.render_target.replace(target_color);
    //     self.main_depth.replace(target_depth);
    // }

}
