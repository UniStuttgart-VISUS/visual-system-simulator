use crate::*;
use std::cell::RefCell;
use std::iter;
use std::rc::Rc;
use std::time::Instant;
use wgpu::{self, SurfaceError, SurfaceTexture};

/// Represents a rendering surface and its associated [Flow].
pub struct Surface {
    surface: wgpu::Surface,
    surface_size: [u32; 2],
    surface_config: RefCell<wgpu::SurfaceConfiguration>,
    device: RefCell<wgpu::Device>,
    queue: RefCell<wgpu::Queue>,

    pub flows: Vec<Flow>,
    vis_param: RefCell<VisualizationParameters>,
    last_render_instant: RefCell<Instant>,
}

impl Surface {
    pub async fn new<W>(surface_size: [u32; 2], window_handle: W, flow_count: usize) -> Self
    where
        W: raw_window_handle::HasRawWindowHandle + raw_window_handle::HasRawDisplayHandle,
    {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            #[cfg(target_os = "macos")]
            backends: wgpu::Backends::METAL,
            #[cfg(not(target_os = "macos"))]
            backends: wgpu::Backends::VULKAN,
            dx12_shader_compiler: wgpu::Dx12Compiler::Fxc,
        });
        let surface = unsafe { instance.create_surface(&window_handle) }.unwrap();
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .unwrap();

        let (device, queue) = adapter
            .request_device(
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
            )
            .await
            .unwrap();

        // Query surface capablities, preferably with sRGB support.
        let swapchain_capabilities = surface.get_capabilities(&adapter);
        let view_formats = vec![];
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
            #[cfg(not(target_os = "android"))]
            format: swapchain_capabilities.formats[0].remove_srgb_suffix(), // TODO find a better workaround for this (e.g. adjust output format of last node)
            width: surface_size[0],
            height: surface_size[1],
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode: swapchain_capabilities.alpha_modes[0],
            view_formats,
        };
        surface.configure(&device, &surface_config);

        // Create flows.
        let mut flows = Vec::new();
        flows.resize_with(flow_count, Flow::new);

        //TODO set perspective from values here ?

        Surface {
            surface,
            surface_size,
            surface_config: RefCell::new(surface_config),
            device: RefCell::new(device),
            queue: RefCell::new(queue),
            flows,
            vis_param: RefCell::new(VisualizationParameters::default()),
            last_render_instant: RefCell::new(Instant::now()),
        }
    }

    pub fn add_node(&mut self, node: Box<dyn Node>, flow_index: usize) {
        self.flows[flow_index].add_node(node);
    }

    pub fn replace_node(&mut self, index: usize, node: Box<dyn Node>, flow_index: usize) {
        self.flows[flow_index].replace_node(index, node);
    }

    pub fn resize(&mut self, new_size: [u32; 2]) {
        assert!(new_size[0] > 0 && new_size[1] > 0, "Non-positive size");
        self.surface_size = [new_size[0], new_size[1]];
        self.surface_config.borrow_mut().width = new_size[0];
        self.surface_config.borrow_mut().height = new_size[1];
        self.surface
            .configure(&self.device.borrow_mut(), &self.surface_config.borrow());
    }

    pub fn delta_t(&self) -> f32 {
        if self.vis_param.borrow().bees_flying {
            return self.last_render_instant.borrow().elapsed().as_micros() as f32;
        }
        return 0.0;
    }

    pub fn nodes_lens(&self) -> Vec<usize> {
        self.flows.iter().map(|flow| flow.nodes_len()).collect()
    }

    pub fn negociate_slots(&self) {
        for flow in self.flows.iter() {
            flow.negociate_slots(self);
        }
    }

    pub fn inspect(&self, inspector: &mut dyn Inspector) {
        self.vis_param.borrow_mut().inspect(inspector);

        for (i, flow) in self.flows.iter().enumerate() {
            inspector.begin_flow(i);
            flow.inspect(inspector);
            inspector.end_flow();
        }
    }

    pub fn inspect_flow(&self, inspector: &mut dyn Inspector, flow_index: usize) {
        inspector.begin_flow(flow_index);
        self.flows[flow_index].inspect(inspector);
        inspector.end_flow();
    }

    pub fn device(&self) -> &RefCell<wgpu::Device> {
        &self.device
    }

    pub fn queue(&self) -> &RefCell<wgpu::Queue> {
        &self.queue
    }

    pub fn surface_config(&self) -> &RefCell<wgpu::SurfaceConfiguration> {
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

        let mut encoder =
            self.device
                .borrow_mut()
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Render Encoder"),
                });

        let render_texture = RenderTexture {
            texture: None,
            view: Rc::new(view),
            sampler: Rc::new(sampler),
            view_dimension: wgpu::TextureViewDimension::D2,
            width: self.width(),
            height: self.height(),
            label: "surface render texture".to_string(),
        };

        self.flows
            .iter()
            .for_each(|f| f.render(&self, &mut encoder, &render_texture));

        self.queue.borrow_mut().submit(iter::once(encoder.finish()));
        output.present();
        self.flows.iter().for_each(|f| f.post_render(&self));
        self.last_render_instant.replace(Instant::now());
    }

    pub fn input(&self, flow: &Flow) {
        flow.input(&self.vis_param.borrow());
    }
}
