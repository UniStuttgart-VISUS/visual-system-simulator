use crate::{Backend, View};
use ash::{
    vk::{self, Handle},
    Entry as VulkanEntry,
};
use cgmath::{Matrix4, Quaternion, SquareMatrix, Vector3};
use openxr as xr;
use std::{env, ffi::CStr, fmt, iter, mem, path::PathBuf, rc::Rc, thread, time::Duration};
use vss::{create_sampler_linear, MouseInput, RenderContext, RenderTexture};

#[cfg(target_vendor = "apple")]
use objc2::{rc::Retained, runtime::ProtocolObject};
#[cfg(target_vendor = "apple")]
use objc2_metal::{MTLCommandQueue, MTLDevice, MTLPixelFormat, MTLTexture};

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
            let instance_flags = wgpu::InstanceFlags::empty();
            let instance_extensions = wgpu_hal::vulkan::Instance::desired_extensions(
                &entry,
                target_version,
                instance_flags,
            )
            .map_err(|err| RuntimeError::Vulkan(err.to_string()))?;
            let instance_extension_names = instance_extensions
                .iter()
                .map(|extension| extension.as_ptr())
                .collect::<Vec<_>>();

            let instance_create_info = vk::InstanceCreateInfo::default()
                .application_info(&app_info)
                .enabled_extension_names(&instance_extension_names)
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
            let adapter_entry = self.load_vulkan_entry()?;
            let adapter_instance =
                ash::Instance::load(adapter_entry.static_fn(), vulkan_instance.handle());
            let hal_instance = wgpu_hal::vulkan::Instance::from_raw(
                adapter_entry,
                adapter_instance,
                target_version,
                0,
                None,
                instance_extensions.clone(),
                instance_flags,
                wgpu::MemoryBudgetThresholds::default(),
                false,
                Some(Box::new(|| {})),
            )
            .map_err(|err| RuntimeError::Vulkan(err.to_string()))?;
            let wgpu_instance = wgpu::Instance::from_hal::<wgpu_hal::api::Vulkan>(hal_instance);
            let wgpu_adapter =
                pollster::block_on(wgpu_instance.request_adapter(&wgpu::RequestAdapterOptions {
                    power_preference: wgpu::PowerPreference::HighPerformance,
                    compatible_surface: None,
                    force_fallback_adapter: false,
                }))
                .map_err(|err| RuntimeError::Vulkan(err.to_string()))?;
            let hal_adapter = wgpu_adapter
                .as_hal::<wgpu_hal::api::Vulkan>()
                .ok_or_else(|| RuntimeError::Vulkan("wgpu returned no Vulkan adapter".into()))?;
            if hal_adapter.raw_physical_device() != physical_device {
                return Err(RuntimeError::Vulkan(
                    "wgpu selected a different Vulkan physical device than OpenXR".into(),
                ));
            }
            let device_extensions = hal_adapter.required_device_extensions(wgpu::Features::empty());
            let device_extension_names = device_extensions
                .iter()
                .map(|extension| extension.as_ptr())
                .collect::<Vec<_>>();
            let mut device_features =
                hal_adapter.physical_device_features(&device_extensions, wgpu::Features::empty());
            let device_create_info = device_features.add_to_device_create(
                vk::DeviceCreateInfo::default()
                    .queue_create_infos(&queue_create_infos)
                    .enabled_extension_names(&device_extension_names),
            );

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

            Ok(VulkanRuntime {
                _entry: entry,
                instance: vulkan_instance,
                physical_device,
                device: vulkan_device,
                queue_family_index,
                instance_extensions,
            })
        }
    }

    fn run_vulkan<F>(&self, build_pipeline: F) -> Result<(), RuntimeError>
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
        let (wgpu_device, wgpu_queue) = self.create_vulkan_wgpu(&vulkan)?;
        let first_view = view_configs
            .first()
            .ok_or_else(|| RuntimeError::Vulkan("OpenXR runtime returned no views".to_string()))?;
        let mut context = RenderContext::new(
            [
                first_view.recommended_image_rect_width,
                first_view.recommended_image_rect_height,
            ],
            view_configs.len(),
            wgpu_device,
            wgpu_queue,
        );
        let initial_views = self.initial_views(&view_configs)?;
        build_pipeline(&mut context, &initial_views);
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
                &mut context,
            )?;

            drop(context);
            drop((space, session, frame_waiter, frame_stream));
            vulkan.device.device_wait_idle().map_err(|err| {
                RuntimeError::Vulkan(format!("Failed to wait for Vulkan device idle: {err}"))
            })?;
            vulkan.device.destroy_device(None);
            vulkan.instance.destroy_instance(None);
        }

        Ok(())
    }

    fn create_vulkan_wgpu(
        &self,
        vulkan: &VulkanRuntime,
    ) -> Result<(wgpu::Device, wgpu::Queue), RuntimeError> {
        let entry = unsafe { self.load_vulkan_entry()? };
        let raw_instance =
            unsafe { ash::Instance::load(entry.static_fn(), vulkan.instance.handle()) };
        let hal_instance = unsafe {
            wgpu_hal::vulkan::Instance::from_raw(
                entry,
                raw_instance,
                vk::API_VERSION_1_1,
                0,
                None,
                vulkan.instance_extensions.clone(),
                wgpu::InstanceFlags::empty(),
                wgpu::MemoryBudgetThresholds::default(),
                false,
                Some(Box::new(|| {})),
            )
        }
        .map_err(|err| RuntimeError::Vulkan(format!("Unable to adopt Vulkan instance: {err}")))?;
        let instance = unsafe { wgpu::Instance::from_hal::<wgpu_hal::api::Vulkan>(hal_instance) };
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: None,
            force_fallback_adapter: false,
        }))
        .map_err(|err| RuntimeError::Vulkan(format!("Unable to find Vulkan adapter: {err}")))?;
        let hal_adapter = unsafe { adapter.as_hal::<wgpu_hal::api::Vulkan>() }
            .ok_or_else(|| RuntimeError::Vulkan("wgpu returned no Vulkan adapter".to_string()))?;
        if hal_adapter.raw_physical_device() != vulkan.physical_device {
            return Err(RuntimeError::Vulkan(
                "wgpu selected a different Vulkan physical device than OpenXR".to_string(),
            ));
        }
        let enabled_extensions = hal_adapter.required_device_extensions(wgpu::Features::empty());
        let raw_device =
            unsafe { ash::Device::load(vulkan.instance.fp_v1_0(), vulkan.device.handle()) };
        let hal_device = unsafe {
            hal_adapter.device_from_raw(
                raw_device,
                Some(Box::new(|| {})),
                &enabled_extensions,
                wgpu::Features::empty(),
                &wgpu::Limits::default(),
                &wgpu::MemoryHints::Performance,
                vulkan.queue_family_index,
                0,
            )
        }
        .map_err(|err| RuntimeError::Vulkan(format!("Unable to adopt Vulkan device: {err}")))?;
        drop(hal_adapter);
        unsafe {
            adapter.create_device_from_hal::<wgpu_hal::api::Vulkan>(
                hal_device,
                &Self::wgpu_device_descriptor(),
            )
        }
        .map_err(|err| RuntimeError::Vulkan(format!("Unable to create wgpu device: {err}")))
    }

    #[cfg(target_vendor = "apple")]
    fn run_metal<F>(&self, build_pipeline: F) -> Result<(), RuntimeError>
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
        let (wgpu_device, wgpu_queue) = self.create_metal_wgpu(device, &command_queue)?;
        let first_view = view_configs
            .first()
            .ok_or_else(|| RuntimeError::Metal("OpenXR runtime returned no views".to_string()))?;
        let mut context = RenderContext::new(
            [
                first_view.recommended_image_rect_width,
                first_view.recommended_image_rect_height,
            ],
            view_configs.len(),
            wgpu_device,
            wgpu_queue,
        );
        let initial_views = self.initial_views(&view_configs)?;
        build_pipeline(&mut context, &initial_views);
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
            &mut context,
        )
    }

    #[cfg(target_vendor = "apple")]
    fn create_metal_wgpu(
        &self,
        device: &ProtocolObject<dyn MTLDevice>,
        command_queue: &Retained<ProtocolObject<dyn MTLCommandQueue>>,
    ) -> Result<(wgpu::Device, wgpu::Queue), RuntimeError> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::METAL,
            ..wgpu::InstanceDescriptor::new_without_display_handle_from_env()
        });
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: None,
            force_fallback_adapter: false,
        }))
        .map_err(|err| RuntimeError::Metal(format!("Unable to find Metal adapter: {err}")))?;
        let raw_device = unsafe {
            Retained::retain(device as *const _ as *mut ProtocolObject<dyn MTLDevice>)
                .ok_or_else(|| RuntimeError::Metal("Unable to retain Metal device".to_string()))?
        };
        let hal_device = unsafe {
            wgpu_hal::metal::Device::device_from_raw(raw_device, wgpu::Features::empty())
        };
        let hal_queue =
            unsafe { wgpu_hal::metal::Queue::queue_from_raw(command_queue.clone(), 1.0) };
        unsafe {
            adapter.create_device_from_hal::<wgpu_hal::api::Metal>(
                wgpu_hal::OpenDevice {
                    device: hal_device,
                    queue: hal_queue,
                },
                &Self::wgpu_device_descriptor(),
            )
        }
        .map_err(|err| RuntimeError::Metal(format!("Unable to adopt Metal device: {err}")))
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
        context: &mut RenderContext,
    ) -> Result<(), RuntimeError> {
        let mut swapchain: Option<MetalSwapchain> = None;
        let mut event_storage = xr::EventDataBuffer::new();
        let mut session_running = false;

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
                swapchain = Some(self.create_metal_swapchain(session, view_configs, context)?);
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
            let (_, views) = session
                .locate_views(VIEW_TYPE, frame_state.predicted_display_time, space)
                .map_err(RuntimeError::OpenXr)?;
            self.render_views(context, &views, &swapchain.targets[image_index as usize])?;
            swapchain
                .handle
                .release_image()
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
        }
    }

    #[cfg(target_vendor = "apple")]
    fn create_metal_swapchain(
        &self,
        session: &xr::Session<xr::Metal>,
        view_configs: &[xr::ViewConfigurationView],
        context: &RenderContext,
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
        let targets = images
            .iter()
            .map(|&image| unsafe {
                let raw = Retained::retain(image.cast::<ProtocolObject<dyn MTLTexture>>())
                    .ok_or_else(|| {
                        RuntimeError::Metal("Unable to retain OpenXR Metal texture".to_string())
                    })?;
                let hal_texture = wgpu_hal::metal::Device::texture_from_raw(
                    raw,
                    wgpu::TextureFormat::Rgba8UnormSrgb,
                    objc2_metal::MTLTextureType::Type2DArray,
                    view_configs.len() as u32,
                    1,
                    wgpu_hal::CopyExtent {
                        width: first_view.recommended_image_rect_width,
                        height: first_view.recommended_image_rect_height,
                        depth: 1,
                    },
                );
                let texture = context
                    .device()
                    .create_texture_from_hal::<wgpu_hal::api::Metal>(
                        hal_texture,
                        &Self::xr_texture_descriptor(
                            first_view.recommended_image_rect_width,
                            first_view.recommended_image_rect_height,
                            view_configs.len() as u32,
                        ),
                    );
                Ok(self.create_layer_targets(context, texture, view_configs.len()))
            })
            .collect::<Result<Vec<_>, RuntimeError>>()?;
        Ok(MetalSwapchain {
            handle,
            _images: images,
            targets,
            width: first_view.recommended_image_rect_width,
            height: first_view.recommended_image_rect_height,
        })
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
        context: &mut RenderContext,
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
        let fences = (0..2)
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
                    context,
                )?);
            }
            let swapchain = swapchain.as_mut().unwrap();
            let image_index = swapchain
                .handle
                .acquire_image()
                .map_err(RuntimeError::OpenXr)?;
            let (_, views) = session
                .locate_views(VIEW_TYPE, frame_state.predicted_display_time, space)
                .map_err(RuntimeError::OpenXr)?;

            swapchain
                .handle
                .wait_image(xr::Duration::INFINITE)
                .map_err(RuntimeError::OpenXr)?;
            self.render_views(context, &views, &swapchain.targets[image_index as usize])?;
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
        context: &RenderContext,
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
        let targets = images
            .iter()
            .map(|&image| unsafe {
                let hal_device = context
                    .device()
                    .as_hal::<wgpu_hal::api::Vulkan>()
                    .ok_or_else(|| {
                        RuntimeError::Vulkan("wgpu returned no Vulkan device".to_string())
                    })?;
                let hal_texture = hal_device.texture_from_raw(
                    vk::Image::from_raw(image),
                    &wgpu_hal::TextureDescriptor {
                        label: Some("OpenXR swapchain texture"),
                        size: wgpu::Extent3d {
                            width: resolution.width,
                            height: resolution.height,
                            depth_or_array_layers: view_configs.len() as u32,
                        },
                        mip_level_count: 1,
                        sample_count: 1,
                        dimension: wgpu::TextureDimension::D2,
                        format: wgpu::TextureFormat::Rgba8UnormSrgb,
                        usage: wgpu::TextureUses::COLOR_TARGET,
                        memory_flags: wgpu_hal::MemoryFlags::empty(),
                        view_formats: vec![],
                    },
                    Some(Box::new(|| {})),
                    wgpu_hal::vulkan::TextureMemory::External,
                );
                drop(hal_device);
                let texture = context
                    .device()
                    .create_texture_from_hal::<wgpu_hal::api::Vulkan>(
                        hal_texture,
                        &Self::xr_texture_descriptor(
                            resolution.width,
                            resolution.height,
                            view_configs.len() as u32,
                        ),
                    );
                Ok(self.create_layer_targets(context, texture, view_configs.len()))
            })
            .collect::<Result<Vec<_>, RuntimeError>>()?;
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
            targets,
            resolution,
        })
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

    fn wgpu_device_descriptor() -> wgpu::DeviceDescriptor<'static> {
        wgpu::DeviceDescriptor {
            label: Some("OpenXR device"),
            required_features: wgpu::Features::empty(),
            required_limits: wgpu::Limits::default(),
            experimental_features: wgpu::ExperimentalFeatures::disabled(),
            memory_hints: wgpu::MemoryHints::Performance,
            trace: wgpu::Trace::Off,
        }
    }

    fn xr_texture_descriptor(
        width: u32,
        height: u32,
        view_count: u32,
    ) -> wgpu::TextureDescriptor<'static> {
        wgpu::TextureDescriptor {
            label: Some("OpenXR swapchain texture"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: view_count,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        }
    }

    fn create_layer_targets(
        &self,
        context: &RenderContext,
        texture: wgpu::Texture,
        view_count: usize,
    ) -> Vec<RenderTexture> {
        let texture = Rc::new(texture);
        (0..view_count)
            .map(|layer| RenderTexture {
                texture: Some(texture.clone()),
                view: Rc::new(texture.create_view(&wgpu::TextureViewDescriptor {
                    label: Some("OpenXR view target"),
                    format: None,
                    dimension: Some(wgpu::TextureViewDimension::D2),
                    usage: Some(wgpu::TextureUsages::RENDER_ATTACHMENT),
                    aspect: wgpu::TextureAspect::All,
                    base_mip_level: 0,
                    mip_level_count: Some(1),
                    base_array_layer: layer as u32,
                    array_layer_count: Some(1),
                })),
                sampler: Rc::new(create_sampler_linear(context.device())),
                view_dimension: wgpu::TextureViewDimension::D2,
                width: context.width(),
                height: context.height(),
                label: format!("OpenXR view {layer}"),
            })
            .collect()
    }

    fn initial_views(
        &self,
        view_configs: &[xr::ViewConfigurationView],
    ) -> Result<Vec<View>, RuntimeError> {
        if view_configs.is_empty() {
            return Err(RuntimeError::View(
                "OpenXR runtime returned no views".to_string(),
            ));
        }
        Ok(view_configs
            .iter()
            .enumerate()
            .map(|(index, config)| View {
                view_index: index,
                eye_index: index.min(1),
                viewport: crate::Viewport {
                    x: 0,
                    y: 0,
                    width: config.recommended_image_rect_width,
                    height: config.recommended_image_rect_height,
                },
                position: Vector3::new(0.0, 0.0, 0.0),
                view: Matrix4::from_scale(1.0),
                projection: cgmath::perspective(cgmath::Deg(70.0), 1.0, 0.05, 1000.0),
            })
            .collect())
    }

    fn render_views(
        &self,
        context: &mut RenderContext,
        located: &[xr::View],
        targets: &[RenderTexture],
    ) -> Result<(), RuntimeError> {
        if located.len() != context.flows.len() || targets.len() != context.flows.len() {
            return Err(RuntimeError::View(format!(
                "OpenXR returned {} views for {} VSS flows and {} render targets",
                located.len(),
                context.flows.len(),
                targets.len()
            )));
        }

        let mut encoder =
            context
                .device()
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("OpenXR VSS render encoder"),
                });
        for (index, (located, target)) in located.iter().zip(targets).enumerate() {
            let view = Self::view_from_openxr(index, located, target.width, target.height)?;
            {
                let mut eye = context.flows[index].eye_mut();
                eye.position = view.position;
                eye.view = view.view;
                eye.proj = view.projection;
            }
            context.flows[index].input(&MouseInput::default());
            context.render_flow(index, &mut encoder, target);
        }
        context.queue().submit(iter::once(encoder.finish()));
        context
            .device()
            .poll(wgpu::PollType::wait_indefinitely())
            .map_err(|err| RuntimeError::View(format!("OpenXR render wait failed: {err}")))?;
        context.post_render();
        Ok(())
    }

    fn view_from_openxr(
        index: usize,
        view: &xr::View,
        width: u32,
        height: u32,
    ) -> Result<View, RuntimeError> {
        let position = Vector3::new(
            view.pose.position.x,
            view.pose.position.y,
            view.pose.position.z,
        );
        let orientation = Quaternion::new(
            view.pose.orientation.w,
            view.pose.orientation.x,
            view.pose.orientation.y,
            view.pose.orientation.z,
        );
        let world_from_view = Matrix4::from_translation(position) * Matrix4::from(orientation);
        let view_matrix = world_from_view.invert().ok_or_else(|| {
            RuntimeError::View("OpenXR returned a singular view pose".to_string())
        })?;
        Ok(View {
            view_index: index,
            eye_index: index.min(1),
            viewport: crate::Viewport {
                x: 0,
                y: 0,
                width,
                height,
            },
            position,
            view: view_matrix,
            projection: Self::projection_from_fov(view.fov, 0.05, 1000.0),
        })
    }

    fn projection_from_fov(fov: xr::Fovf, near: f32, far: f32) -> Matrix4<f32> {
        let left = fov.angle_left.tan();
        let right = fov.angle_right.tan();
        let down = fov.angle_down.tan();
        let up = fov.angle_up.tan();
        let width = right - left;
        let height = up - down;
        Matrix4::new(
            2.0 / width,
            0.0,
            0.0,
            0.0,
            0.0,
            2.0 / height,
            0.0,
            0.0,
            (right + left) / width,
            (up + down) / height,
            -far / (far - near),
            -1.0,
            0.0,
            0.0,
            -(far * near) / (far - near),
            0.0,
        )
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
    queue_family_index: u32,
    instance_extensions: Vec<&'static CStr>,
}

struct VulkanSwapchain {
    handle: xr::Swapchain<xr::Vulkan>,
    buffers: Vec<VulkanFramebuffer>,
    targets: Vec<Vec<RenderTexture>>,
    resolution: vk::Extent2D,
}

struct VulkanFramebuffer {
    framebuffer: vk::Framebuffer,
    color: vk::ImageView,
}

#[cfg(target_vendor = "apple")]
struct MetalSwapchain {
    handle: xr::Swapchain<xr::Metal>,
    _images: Vec<*mut std::ffi::c_void>,
    targets: Vec<Vec<RenderTexture>>,
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
    View(String),
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
            RuntimeError::View(message) => write!(f, "OpenXR view error: {message}"),
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
