use crate::{Backend, View};
use ash::{
    vk::{self, Handle},
    Entry as VulkanEntry,
};
use openxr as xr;
use std::{env, fmt, mem, path::PathBuf};
use vss::RenderContext;

pub const LOADER_PATH_ENV: &str = "VSS_OPENXR_LOADER";
pub const VULKAN_LOADER_PATH_ENV: &str = "VSS_VULKAN_LOADER";
const VULKAN_ICD_FILENAMES_ENV: &str = "VK_ICD_FILENAMES";
const META_XR_SIMULATOR_VULKAN_LOADER: &str =
    "/Applications/MetaXRSimulator.app/Contents/Frameworks/libvulkan.dylib";
const META_XR_SIMULATOR_VULKAN_ICD: &str =
    "/Applications/MetaXRSimulator.app/Contents/Resources/vulkan/icd.d/MoltenVK_icd.json";

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

    pub fn run<F>(&self, _build_pipeline: F) -> Result<(), RuntimeError>
    where
        F: FnOnce(&mut RenderContext, &[View]),
    {
        let info = self.probe()?;
        Err(RuntimeError::NotImplemented {
            backend: info.backend,
            runtime_name: info.runtime_name,
        })
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

    #[allow(dead_code)]
    fn create_vulkan_session(
        &self,
        instance: &xr::Instance,
        system: xr::SystemId,
        requirements: &xr::vulkan::Requirements,
    ) -> Result<VulkanSessionInfo, RuntimeError> {
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

            let session_info = xr::vulkan::SessionCreateInfo {
                instance: vulkan_instance.handle().as_raw() as _,
                physical_device: physical_device.as_raw() as _,
                device: vulkan_device.handle().as_raw() as _,
                queue_family_index,
                queue_index: 0,
            };
            let (session, _frame_waiter, _frame_stream) = instance
                .create_session::<xr::Vulkan>(system, &session_info)
                .map_err(RuntimeError::OpenXr)?;
            let swapchain_formats = session
                .enumerate_swapchain_formats()
                .map_err(RuntimeError::OpenXr)?
                .into_iter()
                .map(|format| format as i64)
                .collect();

            Ok(VulkanSessionInfo {
                queue_family_index,
                swapchain_formats,
            })
        }
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
            unsafe { xr::Entry::load_from(&path) }.map_err(|err| RuntimeError::Loader {
                message: err.to_string(),
                loader_path: Some(path),
            })
        } else {
            Ok(xr::Entry::linked())
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

#[derive(Clone, Debug, Eq, PartialEq)]
struct VulkanSessionInfo {
    queue_family_index: u32,
    swapchain_formats: Vec<i64>,
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
