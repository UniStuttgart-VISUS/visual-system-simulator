use crate::{Backend, View};
use ash::{
    vk::{self, Handle},
    Entry as VulkanEntry,
};
use openxr as xr;
use std::{env, fmt, mem, path::PathBuf, thread, time::Duration};
use vss::RenderContext;

#[cfg(target_vendor = "apple")]
use objc2::{rc::Retained, runtime::ProtocolObject};
#[cfg(target_vendor = "apple")]
use objc2_metal::{
    MTLClearColor, MTLCommandBuffer, MTLCommandEncoder, MTLCommandQueue, MTLDevice, MTLLoadAction,
    MTLPixelFormat, MTLRenderPassDescriptor, MTLStoreAction, MTLTexture,
};

pub const LOADER_PATH_ENV: &str = "VSS_OPENXR_LOADER";
pub const VULKAN_LOADER_PATH_ENV: &str = "VSS_VULKAN_LOADER";
const VULKAN_ICD_FILENAMES_ENV: &str = "VK_ICD_FILENAMES";
const META_XR_SIMULATOR_VULKAN_LOADER: &str =
    "/Applications/MetaXRSimulator.app/Contents/Frameworks/libvulkan.dylib";
const META_XR_SIMULATOR_VULKAN_ICD: &str =
    "/Applications/MetaXRSimulator.app/Contents/Resources/vulkan/icd.d/MoltenVK_icd.json";
const VIEW_TYPE: xr::ViewConfigurationType = xr::ViewConfigurationType::PRIMARY_STEREO;
const COLOR_FORMAT: vk::Format = vk::Format::R8G8B8A8_SRGB;
#[cfg(target_vendor = "apple")]
const METAL_COLOR_FORMAT: MTLPixelFormat = MTLPixelFormat::RGBA8Unorm_sRGB;
const PIPELINE_DEPTH: u32 = 2;

#[derive(Clone, Debug)]
pub struct RuntimeOptions {
    pub backend: Backend,
    pub loader_path: Option<PathBuf>,
}

pub struct Runtime {
    options: RuntimeOptions,
}

impl Runtime {
    pub fn new(options: RuntimeOptions) -> Self {
        Self { options }
    }

    pub fn run<F>(&self, build_pipeline: F) -> Result<(), RuntimeError>
    where
        F: FnOnce(&mut RenderContext, &[View]),
    {
        let entry = self.load_entry()?;

        #[cfg(target_os = "android")]
        entry
            .initialize_android_loader()
            .map_err(RuntimeError::OpenXr)?;

        let available_extensions = entry.enumerate_extensions().map_err(RuntimeError::OpenXr)?;
        let backend = self.select_backend(&available_extensions)?;
        match backend {
            Backend::Vulkan => self.run_vulkan(build_pipeline),
            #[cfg(target_vendor = "apple")]
            Backend::Metal => self.run_metal(build_pipeline),
            #[cfg(not(target_vendor = "apple"))]
            Backend::Metal => Err(RuntimeError::NotImplemented {
                backend: Backend::Metal,
                runtime_name: "this platform".to_string(),
            }),
            Backend::Auto => unreachable!("auto must be resolved before running"),
        }
    }

    pub fn probe(&self) -> Result<RuntimeInfo, RuntimeError> {
        let entry = self.load_entry()?;

        #[cfg(target_os = "android")]
        entry
            .initialize_android_loader()
            .map_err(RuntimeError::OpenXr)?;

        let available_extensions = entry.enumerate_extensions().map_err(RuntimeError::OpenXr)?;
        let backend = self.select_backend(&available_extensions)?;
        let enabled_extensions = self.enabled_extensions(backend);

        let instance = entry
            .create_instance(
                &xr::ApplicationInfo {
                    application_name: "Visual System Simulator",
                    engine_name: "vss-openxr",
                    ..Default::default()
                },
                &enabled_extensions,
                &[],
                &(),
            )
            .map_err(RuntimeError::OpenXr)?;

        let instance_properties = instance.properties().map_err(RuntimeError::OpenXr)?;
        let system = instance
            .system(xr::FormFactor::HEAD_MOUNTED_DISPLAY)
            .map_err(RuntimeError::OpenXr)?;
        let system_properties = instance
            .system_properties(system)
            .map_err(RuntimeError::OpenXr)?;
        let views = instance
            .enumerate_view_configuration_views(system, xr::ViewConfigurationType::PRIMARY_STEREO)
            .map_err(RuntimeError::OpenXr)?
            .into_iter()
            .map(|view| ViewConfigurationView {
                recommended_image_rect_width: view.recommended_image_rect_width,
                recommended_image_rect_height: view.recommended_image_rect_height,
                recommended_swapchain_sample_count: view.recommended_swapchain_sample_count,
            })
            .collect();
        let graphics = match backend {
            Backend::Vulkan => {
                let requirements = instance
                    .graphics_requirements::<xr::Vulkan>(system)
                    .map_err(RuntimeError::OpenXr)?;
                GraphicsInfo::Vulkan {
                    min_api_version_supported: requirements.min_api_version_supported.to_string(),
                    max_api_version_supported: requirements.max_api_version_supported.to_string(),
                }
            }
            Backend::Metal => GraphicsInfo::Metal,
            Backend::Auto => unreachable!("auto must be resolved before probing graphics info"),
        };

        Ok(RuntimeInfo {
            backend,
            runtime_name: instance_properties.runtime_name,
            runtime_version: instance_properties.runtime_version.to_string(),
            system_name: system_properties.system_name,
            views,
            graphics,
        })
    }

    fn select_backend(
        &self,
        available_extensions: &xr::ExtensionSet,
    ) -> Result<Backend, RuntimeError> {
        match self.options.backend {
            Backend::Auto => {
                if available_extensions.khr_vulkan_enable2 {
                    Ok(Backend::Vulkan)
                } else if available_extensions.khr_metal_enable {
                    Ok(Backend::Metal)
                } else {
                    Err(RuntimeError::UnsupportedBackend {
                        requested: Backend::Auto,
                        vulkan_available: available_extensions.khr_vulkan_enable2,
                        metal_available: available_extensions.khr_metal_enable,
                    })
                }
            }
            Backend::Vulkan if available_extensions.khr_vulkan_enable2 => Ok(Backend::Vulkan),
            Backend::Metal if available_extensions.khr_metal_enable => Ok(Backend::Metal),
            requested => Err(RuntimeError::UnsupportedBackend {
                requested,
                vulkan_available: available_extensions.khr_vulkan_enable2,
                metal_available: available_extensions.khr_metal_enable,
            }),
        }
    }

    fn create_vulkan_session(
        &self,
        instance: &xr::Instance,
        system: xr::SystemId,
        requirements: &xr::vulkan::Requirements,
    ) -> Result<VulkanRuntime, RuntimeError> {
        let target_version = vk::make_api_version(0, 1, 1, 0);
        let target_version_xr = xr::Version::new(1, 1, 0);

        if target_version_xr < requirements.min_api_version_supported
            || target_version_xr.major() > requirements.max_api_version_supported.major()
        {
            return Err(RuntimeError::Vulkan(format!(
                "OpenXR runtime requires Vulkan >= {}, < {}.0.0",
                requirements.min_api_version_supported,
                requirements.max_api_version_supported.major() + 1
            )));
        }

        unsafe {
            let entry = self.load_vulkan_entry()?;
            let app_info = vk::ApplicationInfo::default()
                .application_version(0)
                .engine_version(0)
                .api_version(target_version);

            let instance_create_info = vk::InstanceCreateInfo::default()
                .application_info(&app_info)
                .flags(vk::InstanceCreateFlags::ENUMERATE_PORTABILITY_KHR);
            let raw_instance = instance
                .create_vulkan_instance(
                    system,
                    mem::transmute(entry.static_fn().get_instance_proc_addr),
                    &instance_create_info as *const _ as *const _,
                )
                .map_err(RuntimeError::OpenXr)?
                .map_err(vk::Result::from_raw)
                .map_err(|err| RuntimeError::Vulkan(err.to_string()))?;
            let vulkan_instance =
                ash::Instance::load(entry.static_fn(), vk::Instance::from_raw(raw_instance as _));

            let physical_device = vk::PhysicalDevice::from_raw(
                instance
                    .vulkan_graphics_device(system, vulkan_instance.handle().as_raw() as _)
                    .map_err(RuntimeError::OpenXr)? as _,
            );

            let properties = vulkan_instance.get_physical_device_properties(physical_device);
            if properties.api_version < target_version {
                return Err(RuntimeError::Vulkan(
                    "OpenXR-selected Vulkan device does not support Vulkan 1.1".to_string(),
                ));
            }

            let queue_family_index = vulkan_instance
                .get_physical_device_queue_family_properties(physical_device)
                .into_iter()
                .enumerate()
                .find_map(|(index, info)| {
                    if info.queue_flags.contains(vk::QueueFlags::GRAPHICS) {
                        Some(index as u32)
                    } else {
                        None
                    }
                })
                .ok_or_else(|| {
                    RuntimeError::Vulkan(
                        "OpenXR-selected Vulkan device has no graphics queue".to_string(),
                    )
                })?;

            let queue_priorities = [1.0];
            let queue_create_infos = [vk::DeviceQueueCreateInfo::default()
                .queue_family_index(queue_family_index)
                .queue_priorities(&queue_priorities)];
            let mut multiview_features = vk::PhysicalDeviceMultiviewFeatures {
                multiview: vk::TRUE,
                ..Default::default()
            };
            let device_create_info = vk::DeviceCreateInfo::default()
                .queue_create_infos(&queue_create_infos)
                .push_next(&mut multiview_features);

            let raw_device = instance
                .create_vulkan_device(
                    system,
                    mem::transmute(entry.static_fn().get_instance_proc_addr),
                    physical_device.as_raw() as _,
                    &device_create_info as *const _ as *const _,
                )
                .map_err(RuntimeError::OpenXr)?
                .map_err(vk::Result::from_raw)
                .map_err(|err| RuntimeError::Vulkan(err.to_string()))?;
            let vulkan_device = ash::Device::load(
                vulkan_instance.fp_v1_0(),
                vk::Device::from_raw(raw_device as _),
            );

            let queue = vulkan_device.get_device_queue(queue_family_index, 0);

            Ok(VulkanRuntime {
                _entry: entry,
                instance: vulkan_instance,
                physical_device,
                device: vulkan_device,
                queue,
                queue_family_index,
            })
        }
    }

    fn run_vulkan<F>(&self, _build_pipeline: F) -> Result<(), RuntimeError>
    where
        F: FnOnce(&mut RenderContext, &[View]),
    {
        let entry = self.load_entry()?;

        #[cfg(target_os = "android")]
        entry
            .initialize_android_loader()
            .map_err(RuntimeError::OpenXr)?;

        let available_extensions = entry.enumerate_extensions().map_err(RuntimeError::OpenXr)?;
        let backend = self.select_backend(&available_extensions)?;
        if backend != Backend::Vulkan {
            return Err(RuntimeError::UnsupportedBackend {
                requested: Backend::Vulkan,
                vulkan_available: available_extensions.khr_vulkan_enable2,
                metal_available: available_extensions.khr_metal_enable,
            });
        }

        let instance = entry
            .create_instance(
                &xr::ApplicationInfo {
                    application_name: "Visual System Simulator",
                    engine_name: "vss-openxr",
                    ..Default::default()
                },
                &self.enabled_extensions(Backend::Vulkan),
                &[],
                &(),
            )
            .map_err(RuntimeError::OpenXr)?;

        let system = instance
            .system(xr::FormFactor::HEAD_MOUNTED_DISPLAY)
            .map_err(RuntimeError::OpenXr)?;
        let environment_blend_mode = instance
            .enumerate_environment_blend_modes(system, VIEW_TYPE)
            .map_err(RuntimeError::OpenXr)?
            .into_iter()
            .next()
            .unwrap_or(xr::EnvironmentBlendMode::OPAQUE);
        let view_configs = instance
            .enumerate_view_configuration_views(system, VIEW_TYPE)
            .map_err(RuntimeError::OpenXr)?;
        let requirements = instance
            .graphics_requirements::<xr::Vulkan>(system)
            .map_err(RuntimeError::OpenXr)?;
        let vulkan = self.create_vulkan_session(&instance, system, &requirements)?;
        let (session, mut frame_waiter, mut frame_stream) = unsafe {
            instance.create_session::<xr::Vulkan>(
                system,
                &xr::vulkan::SessionCreateInfo {
                    instance: vulkan.instance.handle().as_raw() as _,
                    physical_device: vulkan.physical_device.as_raw() as _,
                    device: vulkan.device.handle().as_raw() as _,
                    queue_family_index: vulkan.queue_family_index,
                    queue_index: 0,
                },
            )
        }
        .map_err(RuntimeError::OpenXr)?;
        let space = session
            .create_reference_space(xr::ReferenceSpaceType::LOCAL, xr::Posef::IDENTITY)
            .map_err(RuntimeError::OpenXr)?;

        unsafe {
            self.run_vulkan_frames(
                &instance,
                &session,
                &mut frame_waiter,
                &mut frame_stream,
                &space,
                environment_blend_mode,
                &view_configs,
                &vulkan,
            )?;

            drop((space, session, frame_waiter, frame_stream));
            vulkan.device.device_wait_idle().map_err(|err| {
                RuntimeError::Vulkan(format!("Failed to wait for Vulkan device idle: {err}"))
            })?;
            vulkan.device.destroy_device(None);
            vulkan.instance.destroy_instance(None);
        }

        Ok(())
    }

    #[cfg(target_vendor = "apple")]
    fn run_metal<F>(&self, _build_pipeline: F) -> Result<(), RuntimeError>
    where
        F: FnOnce(&mut RenderContext, &[View]),
    {
        let entry = self.load_entry()?;
        let available_extensions = entry.enumerate_extensions().map_err(RuntimeError::OpenXr)?;
        if self.select_backend(&available_extensions)? != Backend::Metal {
            return Err(RuntimeError::UnsupportedBackend {
                requested: Backend::Metal,
                vulkan_available: available_extensions.khr_vulkan_enable2,
                metal_available: available_extensions.khr_metal_enable,
            });
        }

        let instance = entry
            .create_instance(
                &xr::ApplicationInfo {
                    application_name: "Visual System Simulator",
                    engine_name: "vss-openxr",
                    ..Default::default()
                },
                &self.enabled_extensions(Backend::Metal),
                &[],
                &(),
            )
            .map_err(RuntimeError::OpenXr)?;
        let system = instance
            .system(xr::FormFactor::HEAD_MOUNTED_DISPLAY)
            .map_err(RuntimeError::OpenXr)?;
        let environment_blend_mode = instance
            .enumerate_environment_blend_modes(system, VIEW_TYPE)
            .map_err(RuntimeError::OpenXr)?
            .into_iter()
            .next()
            .unwrap_or(xr::EnvironmentBlendMode::OPAQUE);
        let view_configs = instance
            .enumerate_view_configuration_views(system, VIEW_TYPE)
            .map_err(RuntimeError::OpenXr)?;
        let requirements = instance
            .graphics_requirements::<xr::Metal>(system)
            .map_err(RuntimeError::OpenXr)?;
        let device = unsafe {
            &*requirements
                .metal_device
                .cast::<ProtocolObject<dyn MTLDevice>>()
        };
        let command_queue = device.newCommandQueue().ok_or_else(|| {
            RuntimeError::Metal("OpenXR-selected Metal device has no command queue".to_string())
        })?;
        let (session, mut frame_waiter, mut frame_stream) = unsafe {
            instance.create_session::<xr::Metal>(
                system,
                &xr::metal::SessionCreateInfo {
                    command_queue: Retained::as_ptr(&command_queue).cast_mut().cast(),
                },
            )
        }
        .map_err(RuntimeError::OpenXr)?;
        let space = session
            .create_reference_space(xr::ReferenceSpaceType::LOCAL, xr::Posef::IDENTITY)
            .map_err(RuntimeError::OpenXr)?;

        self.run_metal_frames(
            &instance,
            &session,
            &mut frame_waiter,
            &mut frame_stream,
            &space,
            environment_blend_mode,
            &view_configs,
            &command_queue,
        )
    }

    #[cfg(target_vendor = "apple")]
    #[allow(clippy::too_many_arguments)]
    fn run_metal_frames(
        &self,
        instance: &xr::Instance,
        session: &xr::Session<xr::Metal>,
        frame_waiter: &mut xr::FrameWaiter,
        frame_stream: &mut xr::FrameStream<xr::Metal>,
        space: &xr::Space,
        environment_blend_mode: xr::EnvironmentBlendMode,
        view_configs: &[xr::ViewConfigurationView],
        command_queue: &ProtocolObject<dyn MTLCommandQueue>,
    ) -> Result<(), RuntimeError> {
        let mut swapchain: Option<MetalSwapchain> = None;
        let mut event_storage = xr::EventDataBuffer::new();
        let mut session_running = false;
        let mut frame = 0usize;

        loop {
            while let Some(event) = instance
                .poll_event(&mut event_storage)
                .map_err(RuntimeError::OpenXr)?
            {
                match event {
                    xr::Event::SessionStateChanged(event) => match event.state() {
                        xr::SessionState::READY => {
                            session.begin(VIEW_TYPE).map_err(RuntimeError::OpenXr)?;
                            session_running = true;
                        }
                        xr::SessionState::STOPPING => {
                            session.end().map_err(RuntimeError::OpenXr)?;
                            session_running = false;
                        }
                        xr::SessionState::EXITING | xr::SessionState::LOSS_PENDING => return Ok(()),
                        _ => {}
                    },
                    xr::Event::InstanceLossPending(_) => return Ok(()),
                    _ => {}
                }
            }

            if !session_running {
                thread::sleep(Duration::from_millis(100));
                continue;
            }

            let frame_state = frame_waiter.wait().map_err(RuntimeError::OpenXr)?;
            frame_stream.begin().map_err(RuntimeError::OpenXr)?;
            if !frame_state.should_render {
                frame_stream
                    .end(
                        frame_state.predicted_display_time,
                        environment_blend_mode,
                        &[],
                    )
                    .map_err(RuntimeError::OpenXr)?;
                continue;
            }

            if swapchain.is_none() {
                swapchain = Some(self.create_metal_swapchain(session, view_configs)?);
            }
            let swapchain = swapchain.as_mut().unwrap();
            let image_index = swapchain
                .handle
                .acquire_image()
                .map_err(RuntimeError::OpenXr)?;
            swapchain
                .handle
                .wait_image(xr::Duration::INFINITE)
                .map_err(RuntimeError::OpenXr)?;
            self.record_metal_clear(
                command_queue,
                swapchain.images[image_index as usize],
                view_configs.len(),
                frame,
            )?;
            swapchain
                .handle
                .release_image()
                .map_err(RuntimeError::OpenXr)?;

            let (_, views) = session
                .locate_views(VIEW_TYPE, frame_state.predicted_display_time, space)
                .map_err(RuntimeError::OpenXr)?;
            let rect = xr::Rect2Di {
                offset: xr::Offset2Di { x: 0, y: 0 },
                extent: xr::Extent2Di {
                    width: swapchain.width as i32,
                    height: swapchain.height as i32,
                },
            };
            let projection_views = views
                .iter()
                .enumerate()
                .map(|(index, view)| {
                    xr::CompositionLayerProjectionView::new()
                        .pose(view.pose)
                        .fov(view.fov)
                        .sub_image(
                            xr::SwapchainSubImage::new()
                                .swapchain(&swapchain.handle)
                                .image_array_index(index as u32)
                                .image_rect(rect),
                        )
                })
                .collect::<Vec<_>>();
            let projection_layer = xr::CompositionLayerProjection::new()
                .space(space)
                .views(&projection_views);
            frame_stream
                .end(
                    frame_state.predicted_display_time,
                    environment_blend_mode,
                    &[&projection_layer],
                )
                .map_err(RuntimeError::OpenXr)?;
            frame = (frame + 1) % PIPELINE_DEPTH as usize;
        }
    }

    #[cfg(target_vendor = "apple")]
    fn create_metal_swapchain(
        &self,
        session: &xr::Session<xr::Metal>,
        view_configs: &[xr::ViewConfigurationView],
    ) -> Result<MetalSwapchain, RuntimeError> {
        let first_view = view_configs
            .first()
            .ok_or_else(|| RuntimeError::Metal("OpenXR runtime returned no views".to_string()))?;
        let handle = session
            .create_swapchain(&xr::SwapchainCreateInfo {
                create_flags: xr::SwapchainCreateFlags::EMPTY,
                usage_flags: xr::SwapchainUsageFlags::COLOR_ATTACHMENT
                    | xr::SwapchainUsageFlags::SAMPLED,
                format: METAL_COLOR_FORMAT.0 as u64,
                sample_count: 1,
                width: first_view.recommended_image_rect_width,
                height: first_view.recommended_image_rect_height,
                face_count: 1,
                array_size: view_configs.len() as u32,
                mip_count: 1,
            })
            .map_err(RuntimeError::OpenXr)?;
        let images = handle.enumerate_images().map_err(RuntimeError::OpenXr)?;
        Ok(MetalSwapchain {
            handle,
            images,
            width: first_view.recommended_image_rect_width,
            height: first_view.recommended_image_rect_height,
        })
    }

    #[cfg(target_vendor = "apple")]
    fn record_metal_clear(
        &self,
        command_queue: &ProtocolObject<dyn MTLCommandQueue>,
        image: *mut std::ffi::c_void,
        view_count: usize,
        frame: usize,
    ) -> Result<(), RuntimeError> {
        let texture = unsafe { &*image.cast::<ProtocolObject<dyn MTLTexture>>() };
        let descriptor = MTLRenderPassDescriptor::new();
        descriptor.setRenderTargetArrayLength(view_count as _);
        let attachment = unsafe { descriptor.colorAttachments().objectAtIndexedSubscript(0) };
        let t = (frame as f64 * 0.17).sin().abs();
        attachment.setTexture(Some(texture));
        attachment.setLoadAction(MTLLoadAction::Clear);
        attachment.setStoreAction(MTLStoreAction::Store);
        attachment.setClearColor(MTLClearColor {
            red: 0.05 + 0.25 * t,
            green: 0.1,
            blue: 0.35 + 0.35 * (1.0 - t),
            alpha: 1.0,
        });
        let command_buffer = command_queue.commandBuffer().ok_or_else(|| {
            RuntimeError::Metal("Metal failed to allocate a command buffer".to_string())
        })?;
        let encoder = command_buffer
            .renderCommandEncoderWithDescriptor(&descriptor)
            .ok_or_else(|| {
                RuntimeError::Metal("Metal failed to create a render encoder".to_string())
            })?;
        encoder.endEncoding();
        command_buffer.commit();
        command_buffer.waitUntilCompleted();
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    unsafe fn run_vulkan_frames(
        &self,
        instance: &xr::Instance,
        session: &xr::Session<xr::Vulkan>,
        frame_waiter: &mut xr::FrameWaiter,
        frame_stream: &mut xr::FrameStream<xr::Vulkan>,
        space: &xr::Space,
        environment_blend_mode: xr::EnvironmentBlendMode,
        view_configs: &[xr::ViewConfigurationView],
        vulkan: &VulkanRuntime,
    ) -> Result<(), RuntimeError> {
        let render_pass = self.create_vulkan_render_pass(vulkan)?;
        let command_pool = vulkan
            .device
            .create_command_pool(
                &vk::CommandPoolCreateInfo::default()
                    .queue_family_index(vulkan.queue_family_index)
                    .flags(
                        vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER
                            | vk::CommandPoolCreateFlags::TRANSIENT,
                    ),
                None,
            )
            .map_err(|err| RuntimeError::Vulkan(format!("Failed to create command pool: {err}")))?;
        let command_buffers = vulkan
            .device
            .allocate_command_buffers(
                &vk::CommandBufferAllocateInfo::default()
                    .command_pool(command_pool)
                    .command_buffer_count(PIPELINE_DEPTH),
            )
            .map_err(|err| {
                RuntimeError::Vulkan(format!("Failed to allocate command buffers: {err}"))
            })?;
        let fences = (0..PIPELINE_DEPTH)
            .map(|_| {
                vulkan.device.create_fence(
                    &vk::FenceCreateInfo::default().flags(vk::FenceCreateFlags::SIGNALED),
                    None,
                )
            })
            .collect::<Result<Vec<_>, _>>()
            .map_err(|err| RuntimeError::Vulkan(format!("Failed to create fences: {err}")))?;

        let mut swapchain: Option<VulkanSwapchain> = None;
        let mut event_storage = xr::EventDataBuffer::new();
        let mut session_running = false;
        let mut frame = 0usize;

        'frames: loop {
            while let Some(event) = instance
                .poll_event(&mut event_storage)
                .map_err(RuntimeError::OpenXr)?
            {
                match event {
                    xr::Event::SessionStateChanged(event) => match event.state() {
                        xr::SessionState::READY => {
                            session.begin(VIEW_TYPE).map_err(RuntimeError::OpenXr)?;
                            session_running = true;
                        }
                        xr::SessionState::STOPPING => {
                            session.end().map_err(RuntimeError::OpenXr)?;
                            session_running = false;
                        }
                        xr::SessionState::EXITING | xr::SessionState::LOSS_PENDING => break 'frames,
                        _ => {}
                    },
                    xr::Event::InstanceLossPending(_) => break 'frames,
                    _ => {}
                }
            }

            if !session_running {
                thread::sleep(Duration::from_millis(100));
                continue;
            }

            let frame_state = frame_waiter.wait().map_err(RuntimeError::OpenXr)?;
            frame_stream.begin().map_err(RuntimeError::OpenXr)?;

            if !frame_state.should_render {
                frame_stream
                    .end(
                        frame_state.predicted_display_time,
                        environment_blend_mode,
                        &[],
                    )
                    .map_err(RuntimeError::OpenXr)?;
                continue;
            }

            if swapchain.is_none() {
                swapchain = Some(self.create_vulkan_swapchain(
                    session,
                    vulkan,
                    render_pass,
                    view_configs,
                )?);
            }
            let swapchain = swapchain.as_mut().unwrap();
            let image_index = swapchain
                .handle
                .acquire_image()
                .map_err(RuntimeError::OpenXr)?;
            let fence = fences[frame];
            vulkan
                .device
                .wait_for_fences(&[fence], true, u64::MAX)
                .map_err(|err| RuntimeError::Vulkan(format!("Failed to wait for fence: {err}")))?;
            vulkan
                .device
                .reset_fences(&[fence])
                .map_err(|err| RuntimeError::Vulkan(format!("Failed to reset fence: {err}")))?;

            let command_buffer = command_buffers[frame];
            self.record_vulkan_clear(
                vulkan,
                command_buffer,
                render_pass,
                swapchain,
                image_index as usize,
                frame,
            )?;
            let (_, views) = session
                .locate_views(VIEW_TYPE, frame_state.predicted_display_time, space)
                .map_err(RuntimeError::OpenXr)?;

            swapchain
                .handle
                .wait_image(xr::Duration::INFINITE)
                .map_err(RuntimeError::OpenXr)?;
            vulkan
                .device
                .queue_submit(
                    vulkan.queue,
                    &[vk::SubmitInfo::default().command_buffers(&[command_buffer])],
                    fence,
                )
                .map_err(|err| RuntimeError::Vulkan(format!("Failed to submit queue: {err}")))?;
            swapchain
                .handle
                .release_image()
                .map_err(RuntimeError::OpenXr)?;

            let rect = xr::Rect2Di {
                offset: xr::Offset2Di { x: 0, y: 0 },
                extent: xr::Extent2Di {
                    width: swapchain.resolution.width as _,
                    height: swapchain.resolution.height as _,
                },
            };
            let mut projection_views = Vec::with_capacity(views.len());
            for (index, view) in views.iter().enumerate() {
                projection_views.push(
                    xr::CompositionLayerProjectionView::new()
                        .pose(view.pose)
                        .fov(view.fov)
                        .sub_image(
                            xr::SwapchainSubImage::new()
                                .swapchain(&swapchain.handle)
                                .image_array_index(index as u32)
                                .image_rect(rect),
                        ),
                );
            }
            let projection_layer = xr::CompositionLayerProjection::new()
                .space(space)
                .views(&projection_views);
            frame_stream
                .end(
                    frame_state.predicted_display_time,
                    environment_blend_mode,
                    &[&projection_layer],
                )
                .map_err(RuntimeError::OpenXr)?;
            frame = (frame + 1) % PIPELINE_DEPTH as usize;
        }

        self.destroy_vulkan_frame_resources(vulkan, render_pass, command_pool, fences, swapchain);
        Ok(())
    }

    unsafe fn create_vulkan_render_pass(
        &self,
        vulkan: &VulkanRuntime,
    ) -> Result<vk::RenderPass, RuntimeError> {
        let view_mask = !(!0 << 2);
        vulkan
            .device
            .create_render_pass(
                &vk::RenderPassCreateInfo::default()
                    .attachments(&[vk::AttachmentDescription {
                        format: COLOR_FORMAT,
                        samples: vk::SampleCountFlags::TYPE_1,
                        load_op: vk::AttachmentLoadOp::CLEAR,
                        store_op: vk::AttachmentStoreOp::STORE,
                        initial_layout: vk::ImageLayout::UNDEFINED,
                        final_layout: vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
                        ..Default::default()
                    }])
                    .subpasses(&[vk::SubpassDescription::default()
                        .color_attachments(&[vk::AttachmentReference {
                            attachment: 0,
                            layout: vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
                        }])
                        .pipeline_bind_point(vk::PipelineBindPoint::GRAPHICS)])
                    .dependencies(&[vk::SubpassDependency {
                        src_subpass: vk::SUBPASS_EXTERNAL,
                        dst_subpass: 0,
                        src_stage_mask: vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
                        dst_stage_mask: vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
                        dst_access_mask: vk::AccessFlags::COLOR_ATTACHMENT_WRITE,
                        ..Default::default()
                    }])
                    .push_next(
                        &mut vk::RenderPassMultiviewCreateInfo::default()
                            .view_masks(&[view_mask])
                            .correlation_masks(&[view_mask]),
                    ),
                None,
            )
            .map_err(|err| RuntimeError::Vulkan(format!("Failed to create render pass: {err}")))
    }

    unsafe fn create_vulkan_swapchain(
        &self,
        session: &xr::Session<xr::Vulkan>,
        vulkan: &VulkanRuntime,
        render_pass: vk::RenderPass,
        view_configs: &[xr::ViewConfigurationView],
    ) -> Result<VulkanSwapchain, RuntimeError> {
        let first_view = view_configs
            .first()
            .ok_or_else(|| RuntimeError::Vulkan("OpenXR runtime returned no views".to_string()))?;
        let resolution = vk::Extent2D {
            width: first_view.recommended_image_rect_width,
            height: first_view.recommended_image_rect_height,
        };
        let handle = session
            .create_swapchain(&xr::SwapchainCreateInfo {
                create_flags: xr::SwapchainCreateFlags::EMPTY,
                usage_flags: xr::SwapchainUsageFlags::COLOR_ATTACHMENT
                    | xr::SwapchainUsageFlags::SAMPLED,
                format: COLOR_FORMAT.as_raw() as _,
                sample_count: 1,
                width: resolution.width,
                height: resolution.height,
                face_count: 1,
                array_size: view_configs.len() as u32,
                mip_count: 1,
            })
            .map_err(RuntimeError::OpenXr)?;

        let images = handle.enumerate_images().map_err(RuntimeError::OpenXr)?;
        let buffers = images
            .into_iter()
            .map(|image| {
                let color_image = vk::Image::from_raw(image);
                let color = vulkan
                    .device
                    .create_image_view(
                        &vk::ImageViewCreateInfo::default()
                            .image(color_image)
                            .view_type(vk::ImageViewType::TYPE_2D_ARRAY)
                            .format(COLOR_FORMAT)
                            .subresource_range(vk::ImageSubresourceRange {
                                aspect_mask: vk::ImageAspectFlags::COLOR,
                                base_mip_level: 0,
                                level_count: 1,
                                base_array_layer: 0,
                                layer_count: view_configs.len() as u32,
                            }),
                        None,
                    )
                    .map_err(|err| {
                        RuntimeError::Vulkan(format!("Failed to create image view: {err}"))
                    })?;
                let framebuffer = vulkan
                    .device
                    .create_framebuffer(
                        &vk::FramebufferCreateInfo::default()
                            .render_pass(render_pass)
                            .width(resolution.width)
                            .height(resolution.height)
                            .attachments(&[color])
                            .layers(1),
                        None,
                    )
                    .map_err(|err| {
                        RuntimeError::Vulkan(format!("Failed to create framebuffer: {err}"))
                    })?;
                Ok(VulkanFramebuffer { framebuffer, color })
            })
            .collect::<Result<Vec<_>, RuntimeError>>()?;

        Ok(VulkanSwapchain {
            handle,
            buffers,
            resolution,
        })
    }

    unsafe fn record_vulkan_clear(
        &self,
        vulkan: &VulkanRuntime,
        command_buffer: vk::CommandBuffer,
        render_pass: vk::RenderPass,
        swapchain: &VulkanSwapchain,
        image_index: usize,
        frame_index: usize,
    ) -> Result<(), RuntimeError> {
        vulkan
            .device
            .begin_command_buffer(
                command_buffer,
                &vk::CommandBufferBeginInfo::default()
                    .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT),
            )
            .map_err(|err| {
                RuntimeError::Vulkan(format!("Failed to begin command buffer: {err}"))
            })?;
        let t = (frame_index as f32 * 0.17).sin().abs();
        vulkan.device.cmd_begin_render_pass(
            command_buffer,
            &vk::RenderPassBeginInfo::default()
                .render_pass(render_pass)
                .framebuffer(swapchain.buffers[image_index].framebuffer)
                .render_area(vk::Rect2D {
                    offset: vk::Offset2D { x: 0, y: 0 },
                    extent: swapchain.resolution,
                })
                .clear_values(&[vk::ClearValue {
                    color: vk::ClearColorValue {
                        float32: [0.05 + 0.25 * t, 0.1, 0.35 + 0.35 * (1.0 - t), 1.0],
                    },
                }]),
            vk::SubpassContents::INLINE,
        );
        vulkan.device.cmd_end_render_pass(command_buffer);
        vulkan
            .device
            .end_command_buffer(command_buffer)
            .map_err(|err| RuntimeError::Vulkan(format!("Failed to end command buffer: {err}")))
    }

    unsafe fn destroy_vulkan_frame_resources(
        &self,
        vulkan: &VulkanRuntime,
        render_pass: vk::RenderPass,
        command_pool: vk::CommandPool,
        fences: Vec<vk::Fence>,
        swapchain: Option<VulkanSwapchain>,
    ) {
        let _ = vulkan.device.wait_for_fences(&fences, true, u64::MAX);
        if let Some(swapchain) = swapchain {
            for buffer in swapchain.buffers {
                vulkan.device.destroy_framebuffer(buffer.framebuffer, None);
                vulkan.device.destroy_image_view(buffer.color, None);
            }
        }
        for fence in fences {
            vulkan.device.destroy_fence(fence, None);
        }
        vulkan.device.destroy_command_pool(command_pool, None);
        vulkan.device.destroy_render_pass(render_pass, None);
    }

    unsafe fn load_vulkan_entry(&self) -> Result<VulkanEntry, RuntimeError> {
        let loader_config = self.vulkan_loader_config();

        if let Some(icd_path) = loader_config.icd_path.as_ref() {
            if env::var_os(VULKAN_ICD_FILENAMES_ENV).is_none() {
                env::set_var(VULKAN_ICD_FILENAMES_ENV, icd_path);
            }
        }

        if let Some(path) = loader_config.loader_path {
            unsafe { VulkanEntry::load_from(&path) }.map_err(|err| {
                RuntimeError::Vulkan(format!(
                    "Unable to load Vulkan loader from {}: {}",
                    path.display(),
                    err
                ))
            })
        } else {
            unsafe { VulkanEntry::load() }.map_err(|err| {
                RuntimeError::Vulkan(format!(
                    "{}. Set {}=/path/to/libvulkan.dylib to use a non-standard Vulkan loader path.",
                    err, VULKAN_LOADER_PATH_ENV
                ))
            })
        }
    }

    fn vulkan_loader_config(&self) -> VulkanLoaderConfig {
        let loader_path = env::var_os(VULKAN_LOADER_PATH_ENV)
            .map(PathBuf::from)
            .or_else(|| {
                env::var_os("VULKAN_SDK").map(|sdk| {
                    let mut path = PathBuf::from(sdk);
                    path.push("lib");
                    path.push("libvulkan.dylib");
                    path
                })
            })
            .filter(|path| path.exists());

        if loader_path.is_some() {
            return VulkanLoaderConfig {
                loader_path,
                icd_path: None,
            };
        }

        let meta_loader_path = {
            let path = PathBuf::from(META_XR_SIMULATOR_VULKAN_LOADER);
            path.exists().then_some(path)
        };
        let meta_icd_path = PathBuf::from(META_XR_SIMULATOR_VULKAN_ICD);

        VulkanLoaderConfig {
            loader_path: meta_loader_path,
            icd_path: meta_icd_path.exists().then_some(meta_icd_path),
        }
    }

    fn enabled_extensions(&self, backend: Backend) -> xr::ExtensionSet {
        let mut extensions = xr::ExtensionSet::default();

        match backend {
            Backend::Vulkan => extensions.khr_vulkan_enable2 = true,
            Backend::Metal => extensions.khr_metal_enable = true,
            Backend::Auto => {}
        }

        #[cfg(target_os = "android")]
        {
            extensions.khr_android_create_instance = true;
        }

        extensions
    }

    fn load_entry(&self) -> Result<xr::Entry, RuntimeError> {
        let loader_path = self
            .options
            .loader_path
            .clone()
            .or_else(|| env::var_os(LOADER_PATH_ENV).map(PathBuf::from));

        if let Some(path) = loader_path {
            unsafe { xr::Entry::load_from(&path, &()) }.map_err(|err| RuntimeError::Loader {
                message: err.to_string(),
                loader_path: Some(path),
            })
        } else {
            xr::Entry::linked(&()).map_err(RuntimeError::OpenXr)
        }
    }
}

#[derive(Clone, Debug)]
pub struct RuntimeInfo {
    pub backend: Backend,
    pub runtime_name: String,
    pub runtime_version: String,
    pub system_name: String,
    pub views: Vec<ViewConfigurationView>,
    pub graphics: GraphicsInfo,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum GraphicsInfo {
    Vulkan {
        min_api_version_supported: String,
        max_api_version_supported: String,
    },
    Metal,
}

struct VulkanRuntime {
    _entry: VulkanEntry,
    instance: ash::Instance,
    physical_device: vk::PhysicalDevice,
    device: ash::Device,
    queue: vk::Queue,
    queue_family_index: u32,
}

struct VulkanSwapchain {
    handle: xr::Swapchain<xr::Vulkan>,
    buffers: Vec<VulkanFramebuffer>,
    resolution: vk::Extent2D,
}

struct VulkanFramebuffer {
    framebuffer: vk::Framebuffer,
    color: vk::ImageView,
}

#[cfg(target_vendor = "apple")]
struct MetalSwapchain {
    handle: xr::Swapchain<xr::Metal>,
    images: Vec<*mut std::ffi::c_void>,
    width: u32,
    height: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct VulkanLoaderConfig {
    loader_path: Option<PathBuf>,
    icd_path: Option<PathBuf>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ViewConfigurationView {
    pub recommended_image_rect_width: u32,
    pub recommended_image_rect_height: u32,
    pub recommended_swapchain_sample_count: u32,
}

#[derive(Debug)]
pub enum RuntimeError {
    Loader {
        message: String,
        loader_path: Option<PathBuf>,
    },
    OpenXr(xr::sys::Result),
    Vulkan(String),
    Metal(String),
    UnsupportedBackend {
        requested: Backend,
        vulkan_available: bool,
        metal_available: bool,
    },
    NotImplemented {
        backend: Backend,
        runtime_name: String,
    },
}

impl fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RuntimeError::Loader {
                message,
                loader_path,
            } => {
                if let Some(path) = loader_path {
                    write!(
                        f,
                        "Unable to load OpenXR loader from {}: {}",
                        path.display(),
                        message
                    )
                } else {
                    write!(
                        f,
                        "Unable to load OpenXR loader: {}. Set {}=/path/to/libopenxr_loader.dylib to use a non-standard loader path.",
                        message, LOADER_PATH_ENV
                    )
                }
            }
            RuntimeError::OpenXr(result) => write!(f, "OpenXR runtime error: {result:?}"),
            RuntimeError::Vulkan(message) => write!(f, "Vulkan initialization error: {message}"),
            RuntimeError::Metal(message) => write!(f, "Metal initialization error: {message}"),
            RuntimeError::UnsupportedBackend {
                requested,
                vulkan_available,
                metal_available,
            } => write!(
                f,
                "OpenXR backend {:?} is not supported by the active runtime (vulkan: {}, metal: {})",
                requested, vulkan_available, metal_available
            ),
            RuntimeError::NotImplemented {
                backend,
                runtime_name,
            } => write!(
                f,
                "OpenXR runtime path for backend {:?} is not implemented yet (runtime: {})",
                backend, runtime_name
            ),
        }
    }
}

impl std::error::Error for RuntimeError {}
