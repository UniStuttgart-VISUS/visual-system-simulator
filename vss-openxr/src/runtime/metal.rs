use super::*;
use objc2::{rc::Retained, runtime::ProtocolObject};
use objc2_metal::{MTLCommandQueue, MTLDevice, MTLPixelFormat, MTLTexture};
use std::{thread, time::Duration};

const METAL_COLOR_FORMAT: MTLPixelFormat = MTLPixelFormat::RGBA8Unorm_sRGB;

struct MetalSwapchain {
    handle: xr::Swapchain<xr::Metal>,
    _images: Vec<*mut std::ffi::c_void>,
    targets: Vec<Vec<RenderTexture>>,
    width: u32,
    height: u32,
}

pub(super) fn run_metal<F>(runtime: &Runtime, build_pipeline: F) -> Result<(), RuntimeError>
where
    F: FnOnce(&mut RenderContext, &[View]) -> Result<(), String>,
{
    let entry = runtime.load_entry()?;
    let available_extensions = entry.enumerate_extensions().map_err(RuntimeError::OpenXr)?;
    if runtime.select_backend(&available_extensions)? != Backend::Metal {
        return Err(RuntimeError::UnsupportedBackend {
            requested: Backend::Metal,
            vulkan_available: available_extensions.khr_vulkan_enable2,
            metal_available: Runtime::metal_backend_supported()
                && available_extensions.khr_metal_enable,
        });
    }

    let instance = entry
        .create_instance(
            &xr::ApplicationInfo {
                application_name: "Visual System Simulator",
                engine_name: "vss-openxr",
                ..Default::default()
            },
            &runtime.enabled_extensions(Backend::Metal, &available_extensions),
            &[],
            &(),
        )
        .map_err(RuntimeError::OpenXr)?;
    let system = instance
        .system(xr::FormFactor::HEAD_MOUNTED_DISPLAY)
        .map_err(RuntimeError::OpenXr)?;
    let view_configuration = runtime.select_view_configuration(&instance, system)?;
    let view_configuration_type = view_configuration.ty;
    let view_configs = view_configuration.views;
    let environment_blend_mode = instance
        .enumerate_environment_blend_modes(system, view_configuration_type)
        .map_err(RuntimeError::OpenXr)?
        .into_iter()
        .next()
        .unwrap_or(xr::EnvironmentBlendMode::OPAQUE);
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
    let (wgpu_device, wgpu_queue) = create_metal_wgpu(runtime, device, &command_queue)?;
    let first_view = view_configs
        .first()
        .ok_or_else(|| RuntimeError::Metal("OpenXR runtime returned no views".to_string()))?;
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
    let initial_views = runtime.initial_views(&view_configs, view_configuration_type)?;
    build_pipeline(&mut context, &initial_views).map_err(RuntimeError::View)?;
    let (session, mut frame_waiter, mut frame_stream) = unsafe {
        instance.create_session::<xr::Metal>(
            system,
            &xr::metal::SessionCreateInfo {
                command_queue: Retained::as_ptr(&command_queue).cast_mut().cast(),
            },
        )
    }
    .map_err(RuntimeError::OpenXr)?;
    let eye_tracking =
        match runtime.create_eye_tracking(&instance, &session, system, &available_extensions) {
            Ok(eye_tracking) => eye_tracking,
            Err(err) => {
                eprintln!("{err}");
                None
            }
        };
    let space = session
        .create_reference_space(xr::ReferenceSpaceType::LOCAL, xr::Posef::IDENTITY)
        .map_err(RuntimeError::OpenXr)?;

    run_metal_frames(
        runtime,
        &instance,
        &session,
        &mut frame_waiter,
        &mut frame_stream,
        &space,
        environment_blend_mode,
        view_configuration_type,
        &view_configs,
        eye_tracking.as_ref(),
        &mut context,
    )
}

fn create_metal_wgpu(
    _runtime: &Runtime,
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
    let hal_device =
        unsafe { wgpu_hal::metal::Device::device_from_raw(raw_device, wgpu::Features::empty()) };
    let hal_queue = unsafe { wgpu_hal::metal::Queue::queue_from_raw(command_queue.clone(), 1.0) };
    unsafe {
        adapter.create_device_from_hal::<wgpu_hal::api::Metal>(
            wgpu_hal::OpenDevice {
                device: hal_device,
                queue: hal_queue,
            },
            &Runtime::wgpu_device_descriptor(),
        )
    }
    .map_err(|err| RuntimeError::Metal(format!("Unable to adopt Metal device: {err}")))
}

#[allow(clippy::too_many_arguments)]
fn run_metal_frames(
    runtime: &Runtime,
    instance: &xr::Instance,
    session: &xr::Session<xr::Metal>,
    frame_waiter: &mut xr::FrameWaiter,
    frame_stream: &mut xr::FrameStream<xr::Metal>,
    space: &xr::Space,
    environment_blend_mode: xr::EnvironmentBlendMode,
    view_configuration_type: xr::ViewConfigurationType,
    view_configs: &[xr::ViewConfigurationView],
    eye_tracking: Option<&EyeTracking>,
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
                        session
                            .begin(view_configuration_type)
                            .map_err(RuntimeError::OpenXr)?;
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
            swapchain = Some(create_metal_swapchain(
                runtime,
                session,
                view_configs,
                context,
            )?);
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
        let (view_state_flags, views) = session
            .locate_views(
                view_configuration_type,
                frame_state.predicted_display_time,
                space,
            )
            .map_err(RuntimeError::OpenXr)?;
        runtime.render_views(
            session,
            eye_tracking,
            context,
            view_state_flags,
            &views,
            &swapchain.targets[image_index as usize],
            frame_state.predicted_display_time,
            space,
        )?;
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

fn create_metal_swapchain(
    runtime: &Runtime,
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
            let raw = Retained::retain(image.cast::<ProtocolObject<dyn MTLTexture>>()).ok_or_else(
                || RuntimeError::Metal("Unable to retain OpenXR Metal texture".to_string()),
            )?;
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
                    &Runtime::xr_texture_descriptor(
                        first_view.recommended_image_rect_width,
                        first_view.recommended_image_rect_height,
                        view_configs.len() as u32,
                    ),
                );
            Ok(runtime.create_layer_targets(context, texture, view_configs.len()))
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
