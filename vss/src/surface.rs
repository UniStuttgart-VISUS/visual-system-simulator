use crate::*;
use std::cell::Cell;
use std::iter;
use std::rc::Rc;
use instant::Instant;
use wgpu::{self, SurfaceError, SurfaceTexture};

/// Represents a rendering surface and its associated [Flow].
pub struct Surface {
    surface: wgpu::Surface,
    surface_size: [u32; 2],
    surface_config: wgpu::SurfaceConfiguration,
    device: wgpu::Device,
    queue: wgpu::Queue,

    pub flows: Vec<Flow>,
    last_render_instant: Cell<Instant>,
}

impl Surface {
    pub async fn new<W>(surface_size: [u32; 2], window_handle: W, flow_count: usize) -> Self
    where
        W: raw_window_handle::HasRawWindowHandle + raw_window_handle::HasRawDisplayHandle,
    {
        let instance = if cfg!(target_os = "windows") {
            // Use Vulkan for consistency with Varjo/OpenXR builds on windows.
            wgpu::Instance::new(wgpu::InstanceDescriptor {
                backends: wgpu::Backends::VULKAN,
                dx12_shader_compiler: wgpu::Dx12Compiler::Fxc,
            })
        } else {
            wgpu::Instance::default()
        };

        let surface = unsafe { instance.create_surface(&window_handle) }.unwrap();
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .expect("Cannot create adapter");

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    features: wgpu::Features::empty(),
                    limits: if cfg!(target_arch = "wasm32") {
                        // WebGL does not support all features, thus disable some.
                        wgpu::Limits::downlevel_webgl2_defaults().using_resolution(adapter.limits())
                    } else {
                        wgpu::Limits::default()
                    },
                    label: None,
                },
                None,
            )
            .await
            .expect("Cannot create device");

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

        Surface {
            surface,
            surface_size,
            surface_config,
            device,
            queue,
            flows,
            last_render_instant: Cell::new(Instant::now()),
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
        self.surface_config.width = new_size[0];
        self.surface_config.height = new_size[1];
        self.surface.configure(&self.device, &self.surface_config);
    }

    pub fn delta_t(&self) -> f32 {
        self.last_render_instant.get().elapsed().as_micros() as f32
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
        for (i, flow) in self.flows.iter().enumerate() {
            inspector.flow(i, &flow);
        }
    }

    pub fn device(&self) -> &wgpu::Device {
        &self.device
    }

    pub fn queue(&self) -> &wgpu::Queue {
        &self.queue
    }

    pub fn width(&self) -> u32 {
        self.surface_size[0]
    }

    pub fn height(&self) -> u32 {
        self.surface_size[1]
    }

    pub fn get_current_texture(&self) -> Result<SurfaceTexture, SurfaceError> {
        self.surface.get_current_texture()
    }

    pub fn draw(&self) {
        let output = self.get_current_texture().unwrap();

        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let sampler = create_sampler_linear(&(self.device));

        let mut encoder = self
            .device
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
            .for_each(|f| f.render(self, &mut encoder, &render_texture));

        self.queue.submit(iter::once(encoder.finish()));
        output.present();
        self.flows.iter().for_each(|f| f.post_render(self));
        self.last_render_instant.replace(Instant::now());
    }
}
