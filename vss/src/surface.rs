use crate::*;
use std::iter;
use std::ops::{Deref, DerefMut};
use std::rc::Rc;
use wgpu::{self, CurrentSurfaceTexture};

#[cfg(target_os = "android")]
const ANDROID_HARDWARE_BUFFER_EXTENSION: &std::ffi::CStr =
    c"VK_ANDROID_external_memory_android_hardware_buffer";

#[cfg(target_os = "android")]
const QUEUE_FAMILY_FOREIGN_EXTENSION: &std::ffi::CStr = c"VK_EXT_queue_family_foreign";

#[cfg(target_os = "android")]
const SAMPLER_YCBCR_CONVERSION_EXTENSION: &std::ffi::CStr = c"VK_KHR_sampler_ycbcr_conversion";

/// Represents a presentation surface and its associated [RenderContext].
pub struct Surface<'window> {
    surface: wgpu::Surface<'window>,
    surface_config: wgpu::SurfaceConfiguration,
    render_context: RenderContext,
}

impl<'window> Surface<'window> {
    pub async fn with_existing(
        surface_size: [u32; 2],
        flow_count: usize,
        surface: wgpu::Surface<'static>,
        adapter: wgpu::Adapter,
        device: wgpu::Device,
        queue: wgpu::Queue,
    ) -> Self {
        // Query surface capablities, preferably with sRGB support.
        let swapchain_capabilities = surface.get_capabilities(&adapter);
        let view_formats = vec![];

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
            desired_maximum_frame_latency: 2,
        };
        let output_format = surface_config.format;
        surface.configure(&device, &surface_config);

        Surface {
            surface,
            surface_config,
            render_context: RenderContext::new(
                surface_size,
                flow_count,
                device,
                queue,
                output_format,
            ),
        }
    }

    pub fn new(
        surface_size: [u32; 2],
        target: impl Into<wgpu::SurfaceTarget<'window>>,
        flow_count: usize,
    ) -> Self {
        let instance = if cfg!(target_os = "windows") {
            // Use Vulkan for consistency with Varjo/OpenXR builds on windows.
            wgpu::Instance::new(wgpu::InstanceDescriptor {
                backends: wgpu::Backends::VULKAN,
                ..wgpu::InstanceDescriptor::new_without_display_handle_from_env()
            })
        } else {
            wgpu::Instance::default()
        };

        let surface = instance.create_surface(target).unwrap();
        let (adapter, device, queue) = pollster::block_on(async {
            let adapter = instance
                .request_adapter(&wgpu::RequestAdapterOptions {
                    power_preference: wgpu::PowerPreference::HighPerformance,
                    compatible_surface: Some(&surface),
                    force_fallback_adapter: false,
                })
                .await
                .expect("Cannot create adapter");

            #[cfg(target_os = "android")]
            let (device, queue) = {
                let descriptor = android_device_descriptor(&adapter);
                let mut ycbcr_features =
                    ash::vk::PhysicalDeviceSamplerYcbcrConversionFeatures::default()
                        .sampler_ycbcr_conversion(true);
                let hal_adapter = unsafe {
                    adapter
                        .as_hal::<wgpu_hal::api::Vulkan>()
                        .expect("Android renderer should use Vulkan")
                };
                let hal_device = unsafe {
                    hal_adapter
                        .open_with_callback(
                            descriptor.required_features,
                            &descriptor.required_limits,
                            &descriptor.memory_hints,
                            Some(Box::new(|args| {
                                push_unique_extension(
                                    args.extensions,
                                    ANDROID_HARDWARE_BUFFER_EXTENSION,
                                );
                                push_unique_extension(
                                    args.extensions,
                                    QUEUE_FAMILY_FOREIGN_EXTENSION,
                                );
                                push_unique_extension(
                                    args.extensions,
                                    SAMPLER_YCBCR_CONVERSION_EXTENSION,
                                );
                                *args.create_info = args.create_info.push_next(&mut ycbcr_features);
                            })),
                        )
                        .expect("Cannot create Android Vulkan device")
                };
                unsafe {
                    adapter
                        .create_device_from_hal::<wgpu_hal::api::Vulkan>(hal_device, &descriptor)
                        .expect("Cannot wrap Android Vulkan device")
                }
            };

            #[cfg(not(target_os = "android"))]
            let (device, queue) = adapter
                .request_device(&wgpu::DeviceDescriptor {
                    label: None,
                    required_features: wgpu::Features::empty(),
                    required_limits: if cfg!(target_arch = "wasm32") {
                        // WebGL does not support all features, thus disable some.
                        wgpu::Limits::downlevel_webgl2_defaults().using_resolution(adapter.limits())
                    } else {
                        wgpu::Limits::default()
                    },
                    experimental_features: wgpu::ExperimentalFeatures::disabled(),
                    trace: wgpu::Trace::Off,
                    memory_hints: wgpu::MemoryHints::Performance,
                })
                .await
                .expect("Cannot create device");
            (adapter, device, queue)
        });
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
            desired_maximum_frame_latency: 2,
        };
        let output_format = surface_config.format;
        surface.configure(&device, &surface_config);

        Surface {
            surface,
            surface_config,
            render_context: RenderContext::new(
                surface_size,
                flow_count,
                device,
                queue,
                output_format,
            ),
        }
    }

    pub fn resize(&mut self, new_size: [u32; 2]) {
        self.render_context.resize(new_size);
        self.surface_config.width = new_size[0];
        self.surface_config.height = new_size[1];
        self.surface
            .configure(self.render_context.device(), &self.surface_config);
    }

    pub fn get_current_texture(&self) -> CurrentSurfaceTexture {
        self.surface.get_current_texture()
    }

    pub fn draw(&self) -> bool {
        let output = match self.get_current_texture() {
            CurrentSurfaceTexture::Success(output) | CurrentSurfaceTexture::Suboptimal(output) => {
                output
            }
            CurrentSurfaceTexture::Timeout | CurrentSurfaceTexture::Occluded => {
                return false;
            }
            other => panic!("Failed to acquire surface texture: {other:?}"),
        };

        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let sampler = create_sampler_linear(self.render_context.device());

        let mut encoder =
            self.render_context
                .device()
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Render Encoder"),
                });

        let render_texture = RenderTexture {
            texture: None,
            view: Rc::new(view),
            sampler: Rc::new(sampler),
            view_dimension: wgpu::TextureViewDimension::D2,
            width: self.render_context.width(),
            height: self.render_context.height(),
            label: "surface render texture".to_string(),
        };

        self.render_context.render(&mut encoder, &render_texture);

        self.render_context
            .queue()
            .submit(iter::once(encoder.finish()));
        output.present();
        self.render_context.post_render();
        true
    }
}

#[cfg(target_os = "android")]
fn android_device_descriptor(adapter: &wgpu::Adapter) -> wgpu::DeviceDescriptor<'static> {
    let mut required_features = wgpu::Features::empty();
    if adapter
        .features()
        .contains(wgpu::Features::TEXTURE_FORMAT_NV12)
    {
        required_features |= wgpu::Features::TEXTURE_FORMAT_NV12;
    }

    wgpu::DeviceDescriptor {
        label: None,
        required_features,
        required_limits: wgpu::Limits::default(),
        experimental_features: wgpu::ExperimentalFeatures::disabled(),
        trace: wgpu::Trace::Off,
        memory_hints: wgpu::MemoryHints::Performance,
    }
}

#[cfg(target_os = "android")]
fn push_unique_extension(
    extensions: &mut Vec<&'static std::ffi::CStr>,
    extension: &'static std::ffi::CStr,
) {
    if !extensions.contains(&extension) {
        extensions.push(extension);
    }
}

impl Deref for Surface<'_> {
    type Target = RenderContext;

    fn deref(&self) -> &Self::Target {
        &self.render_context
    }
}

impl DerefMut for Surface<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.render_context
    }
}
