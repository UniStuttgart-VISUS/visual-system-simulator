use super::*;
use ash::{
    vk::{self, Handle},
    Entry as VulkanEntry,
};
use std::{ffi::CStr, mem, path::PathBuf, thread, time::Duration};

const COLOR_FORMAT: vk::Format = vk::Format::R8G8B8A8_SRGB;

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
    targets: Vec<Vec<RenderTexture>>,
    resolution: vk::Extent2D,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct VulkanLoaderConfig {
    loader_path: Option<PathBuf>,
    icd_path: Option<PathBuf>,
}

pub(super) fn run_vulkan<F>(runtime: &Runtime, build_pipeline: F) -> Result<(), RuntimeError>
where
    F: FnOnce(&mut RenderContext, &[View]),
{
    let entry = runtime.load_entry()?;

    #[cfg(target_os = "android")]
    entry
        .initialize_android_loader()
        .map_err(RuntimeError::OpenXr)?;

    let available_extensions = entry.enumerate_extensions().map_err(RuntimeError::OpenXr)?;
    let backend = runtime.select_backend(&available_extensions)?;
    if backend != Backend::Vulkan {
        return Err(RuntimeError::UnsupportedBackend {
            requested: Backend::Vulkan,
            vulkan_available: available_extensions.khr_vulkan_enable2,
            metal_available: Runtime::metal_backend_available(&available_extensions),
        });
    }

    let instance = entry
        .create_instance(
            &xr::ApplicationInfo {
                application_name: "Visual System Simulator",
                engine_name: "vss-openxr",
                ..Default::default()
            },
            &runtime.enabled_extensions(Backend::Vulkan),
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
    let vulkan = create_vulkan_session(runtime, &instance, system, &requirements)?;
    let (wgpu_device, wgpu_queue) = create_vulkan_wgpu(runtime, &vulkan)?;
    let first_view = view_configs
        .first()
        .ok_or_else(|| RuntimeError::Vulkan("OpenXR runtime returned no views".to_string()))?;
    let output_format = wgpu::TextureFormat::Rgba8UnormSrgb;
    let mut context = RenderContext::new(
        [
            first_view.recommended_image_rect_width,
            first_view.recommended_image_rect_height,
        ],
        view_configs.len(),
        wgpu_device,
        wgpu_queue,
        output_format,
    );
    let initial_views = runtime.initial_views(&view_configs)?;
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
        run_vulkan_frames(
            runtime,
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

fn create_vulkan_session(
    runtime: &Runtime,
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
        let entry = load_vulkan_entry(runtime)?;
        let app_info = vk::ApplicationInfo::default()
            .application_version(0)
            .engine_version(0)
            .api_version(target_version);
        let instance_flags = wgpu::InstanceFlags::empty();
        let instance_extensions =
            wgpu_hal::vulkan::Instance::desired_extensions(&entry, target_version, instance_flags)
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
        let adapter_entry = load_vulkan_entry(runtime)?;
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

fn create_vulkan_wgpu(
    runtime: &Runtime,
    vulkan: &VulkanRuntime,
) -> Result<(wgpu::Device, wgpu::Queue), RuntimeError> {
    let entry = unsafe { load_vulkan_entry(runtime)? };
    let raw_instance = unsafe { ash::Instance::load(entry.static_fn(), vulkan.instance.handle()) };
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
            &Runtime::wgpu_device_descriptor(),
        )
    }
    .map_err(|err| RuntimeError::Vulkan(format!("Unable to create wgpu device: {err}")))
}

#[allow(clippy::too_many_arguments)]
unsafe fn run_vulkan_frames(
    runtime: &Runtime,
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
            swapchain = Some(create_vulkan_swapchain(
                runtime,
                session,
                vulkan,
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
        runtime.render_views(context, &views, &swapchain.targets[image_index as usize])?;
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
    Ok(())
}

unsafe fn create_vulkan_swapchain(
    runtime: &Runtime,
    session: &xr::Session<xr::Vulkan>,
    _vulkan: &VulkanRuntime,
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
                    &Runtime::xr_texture_descriptor(
                        resolution.width,
                        resolution.height,
                        view_configs.len() as u32,
                    ),
                );
            Ok(runtime.create_layer_targets(context, texture, view_configs.len()))
        })
        .collect::<Result<Vec<_>, RuntimeError>>()?;
    Ok(VulkanSwapchain {
        handle,
        targets,
        resolution,
    })
}

unsafe fn load_vulkan_entry(runtime: &Runtime) -> Result<VulkanEntry, RuntimeError> {
    let loader_config = vulkan_loader_config(runtime);

    if let Some(icd_path) = loader_config.icd_path.as_ref() {
        if std::env::var_os(VULKAN_ICD_FILENAMES_ENV).is_none() {
            std::env::set_var(VULKAN_ICD_FILENAMES_ENV, icd_path);
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

fn vulkan_loader_config(runtime: &Runtime) -> VulkanLoaderConfig {
    let loader_path = std::env::var_os(VULKAN_LOADER_PATH_ENV)
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var_os("VULKAN_SDK").map(|sdk| {
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

    let _ = runtime;
    VulkanLoaderConfig {
        loader_path: meta_loader_path,
        icd_path: meta_icd_path.exists().then_some(meta_icd_path),
    }
}
