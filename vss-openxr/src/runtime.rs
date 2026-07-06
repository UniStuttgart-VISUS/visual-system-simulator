#[cfg(target_vendor = "apple")]
mod metal;
mod vulkan;

use crate::{Backend, View};
use cgmath::{InnerSpace, Matrix3, Matrix4, Quaternion, SquareMatrix, Vector3};
use openxr as xr;
use openxr::sys as xrsys;
use std::{env, fmt, iter, path::PathBuf, ptr, rc::Rc};
use vss::{create_sampler_linear, MouseInput, RenderContext, RenderTexture};

pub const LOADER_PATH_ENV: &str = "VSS_OPENXR_LOADER";
pub const VULKAN_LOADER_PATH_ENV: &str = "VSS_VULKAN_LOADER";
const VULKAN_ICD_FILENAMES_ENV: &str = "VK_ICD_FILENAMES";
const META_XR_SIMULATOR_VULKAN_LOADER: &str =
    "/Applications/MetaXRSimulator.app/Contents/Frameworks/libvulkan.dylib";
const META_XR_SIMULATOR_VULKAN_ICD: &str =
    "/Applications/MetaXRSimulator.app/Contents/Resources/vulkan/icd.d/MoltenVK_icd.json";
const DEFAULT_VIEW_TYPE: xr::ViewConfigurationType = xr::ViewConfigurationType::PRIMARY_STEREO;

#[derive(Clone, Debug)]
struct SelectedViewConfiguration {
    ty: xr::ViewConfigurationType,
    views: Vec<xr::ViewConfigurationView>,
}

struct EyeTracking {
    action_set: xr::ActionSet,
    gaze_space: xr::Space,
}

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
        F: FnOnce(&mut RenderContext, &[View]) -> Result<(), String>,
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
        let enabled_extensions = self.enabled_extensions(backend, &available_extensions);

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
        let view_configuration = self.select_view_configuration(&instance, system)?;
        let system_properties = instance
            .system_properties(system)
            .map_err(RuntimeError::OpenXr)?;
        let views = view_configuration
            .views
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
                } else if Self::metal_backend_available(available_extensions) {
                    Ok(Backend::Metal)
                } else {
                    Err(RuntimeError::UnsupportedBackend {
                        requested: Backend::Auto,
                        vulkan_available: available_extensions.khr_vulkan_enable2,
                        metal_available: Self::metal_backend_available(available_extensions),
                    })
                }
            }
            Backend::Vulkan if available_extensions.khr_vulkan_enable2 => Ok(Backend::Vulkan),
            Backend::Metal if Self::metal_backend_available(available_extensions) => {
                Ok(Backend::Metal)
            }
            requested => Err(RuntimeError::UnsupportedBackend {
                requested,
                vulkan_available: available_extensions.khr_vulkan_enable2,
                metal_available: Self::metal_backend_available(available_extensions),
            }),
        }
    }

    #[cfg(target_vendor = "apple")]
    pub(super) fn metal_backend_supported() -> bool {
        cfg!(target_vendor = "apple")
    }

    pub(super) fn metal_backend_available(available_extensions: &xr::ExtensionSet) -> bool {
        #[cfg(target_vendor = "apple")]
        {
            Self::metal_backend_supported() && available_extensions.khr_metal_enable
        }

        #[cfg(not(target_vendor = "apple"))]
        {
            let _ = available_extensions;
            false
        }
    }

    fn run_vulkan<F>(&self, build_pipeline: F) -> Result<(), RuntimeError>
    where
        F: FnOnce(&mut RenderContext, &[View]) -> Result<(), String>,
    {
        vulkan::run_vulkan(self, build_pipeline)
    }

    #[cfg(all(target_vendor = "apple"))]
    fn run_metal<F>(&self, build_pipeline: F) -> Result<(), RuntimeError>
    where
        F: FnOnce(&mut RenderContext, &[View]) -> Result<(), String>,
    {
        metal::run_metal(self, build_pipeline)
    }

    #[cfg(not(all(target_vendor = "apple")))]
    #[allow(dead_code)]
    fn run_metal<F>(&self, _build_pipeline: F) -> Result<(), RuntimeError>
    where
        F: FnOnce(&mut RenderContext, &[View]) -> Result<(), String>,
    {
        Err(RuntimeError::NotImplemented {
            backend: Backend::Metal,
            runtime_name: "this platform".to_string(),
        })
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

    fn select_view_configuration(
        &self,
        instance: &xr::Instance,
        system: xr::SystemId,
    ) -> Result<SelectedViewConfiguration, RuntimeError> {
        let supported = instance
            .enumerate_view_configurations(system)
            .map_err(RuntimeError::OpenXr)?;
        let view_type = [
            xr::ViewConfigurationType::PRIMARY_QUAD_VARJO,
            DEFAULT_VIEW_TYPE,
            xr::ViewConfigurationType::PRIMARY_MONO,
        ]
        .iter()
        .copied()
        .find(|ty| supported.contains(ty))
        .ok_or_else(|| {
            RuntimeError::View(
                "OpenXR runtime returned no supported primary view configuration".to_string(),
            )
        })?;
        let views = instance
            .enumerate_view_configuration_views(system, view_type)
            .map_err(RuntimeError::OpenXr)?;
        let first = views
            .first()
            .ok_or_else(|| RuntimeError::View("OpenXR runtime returned no views".to_string()))?;
        if views.iter().any(|view| {
            view.recommended_image_rect_width != first.recommended_image_rect_width
                || view.recommended_image_rect_height != first.recommended_image_rect_height
                || view.recommended_swapchain_sample_count
                    != first.recommended_swapchain_sample_count
        }) {
            return Err(RuntimeError::View(
                "OpenXR runtime returned mismatched per-view recommendations".to_string(),
            ));
        }
        Ok(SelectedViewConfiguration {
            ty: view_type,
            views,
        })
    }

    fn initial_views(
        &self,
        view_configs: &[xr::ViewConfigurationView],
        view_configuration_type: xr::ViewConfigurationType,
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
                eye_index: Self::eye_index_for_view(view_configuration_type, index),
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

    fn eye_index_for_view(
        view_configuration_type: xr::ViewConfigurationType,
        view_index: usize,
    ) -> usize {
        match view_configuration_type {
            xr::ViewConfigurationType::PRIMARY_QUAD_VARJO => match view_index {
                0 | 1 => 0,
                _ => 1,
            },
            xr::ViewConfigurationType::PRIMARY_MONO => 0,
            _ => match view_index {
                0 => 0,
                1 => 1,
                _ => 1,
            },
        }
    }

    fn eye_index_for_view_count(view_count: usize, view_index: usize) -> usize {
        match view_count {
            4 => match view_index {
                0 | 1 => 0,
                _ => 1,
            },
            2 => match view_index {
                0 => 0,
                _ => 1,
            },
            _ => 0,
        }
    }

    fn create_eye_tracking<G>(
        &self,
        instance: &xr::Instance,
        session: &xr::Session<G>,
        system: xr::SystemId,
        available_extensions: &xr::ExtensionSet,
    ) -> Result<Option<EyeTracking>, RuntimeError> {
        if !available_extensions.ext_eye_gaze_interaction {
            return Ok(None);
        }

        let mut eye_gaze_props = xrsys::SystemEyeGazeInteractionPropertiesEXT::out(ptr::null_mut());
        let mut system_props = xrsys::SystemProperties::out(
            eye_gaze_props.as_mut_ptr() as *mut xrsys::BaseOutStructure
        );
        let result = unsafe {
            (instance.fp().get_system_properties)(
                instance.as_raw(),
                system,
                system_props.as_mut_ptr(),
            )
        };
        if result != xrsys::Result::SUCCESS {
            return Err(RuntimeError::OpenXr(result));
        }
        let eye_gaze_props = unsafe { eye_gaze_props.assume_init() };
        if !bool::from(eye_gaze_props.supports_eye_gaze_interaction) {
            return Ok(None);
        }

        let action_set = match instance.create_action_set("eye_gaze", "Eye gaze", 0) {
            Ok(action_set) => action_set,
            Err(err) => return Err(RuntimeError::OpenXr(err)),
        };
        let eyes_path = instance
            .string_to_path("/user/eyes_ext")
            .map_err(RuntimeError::OpenXr)?;
        let gaze_action = action_set
            .create_action::<xr::Posef>("gaze_pose", "Gaze pose", &[eyes_path])
            .map_err(RuntimeError::OpenXr)?;
        let interaction_profile = instance
            .string_to_path("/interaction_profiles/ext/eye_gaze_interaction")
            .map_err(RuntimeError::OpenXr)?;
        let gaze_path = instance
            .string_to_path("/user/eyes_ext/input/gaze_ext/pose")
            .map_err(RuntimeError::OpenXr)?;
        instance
            .suggest_interaction_profile_bindings(
                interaction_profile,
                &[xr::Binding::new(&gaze_action, gaze_path)],
            )
            .map_err(RuntimeError::OpenXr)?;
        session
            .attach_action_sets(&[&action_set])
            .map_err(RuntimeError::OpenXr)?;
        let gaze_space = gaze_action
            .create_space(session, eyes_path, xr::Posef::IDENTITY)
            .map_err(RuntimeError::OpenXr)?;

        Ok(Some(EyeTracking {
            action_set,
            gaze_space,
        }))
    }

    fn render_views<G>(
        &self,
        session: &xr::Session<G>,
        eye_tracking: Option<&EyeTracking>,
        context: &mut RenderContext,
        view_state_flags: xr::ViewStateFlags,
        located: &[xr::View],
        targets: &[RenderTexture],
        predicted_display_time: xr::Time,
        base_space: &xr::Space,
    ) -> Result<(), RuntimeError> {
        if located.len() != context.flows.len() || targets.len() != context.flows.len() {
            return Err(RuntimeError::View(format!(
                "OpenXR returned {} views for {} VSS flows and {} render targets",
                located.len(),
                context.flows.len(),
                targets.len()
            )));
        }

        if let Some(eye_tracking) = eye_tracking {
            session
                .sync_actions(&[xr::ActiveActionSet::new(&eye_tracking.action_set)])
                .map_err(RuntimeError::OpenXr)?;
        }

        let gaze_vectors = if let Some(eye_tracking) = eye_tracking {
            self.locate_gaze_vectors(
                session,
                eye_tracking,
                predicted_display_time,
                base_space,
                located,
            )?
        } else {
            vec![None; located.len()]
        };
        let hardware_pose_valid = view_state_flags.contains(xr::ViewStateFlags::POSITION_VALID)
            && view_state_flags.contains(xr::ViewStateFlags::ORIENTATION_VALID);

        let mut encoder =
            context
                .device()
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("OpenXR VSS render encoder"),
                });
        let view_count = located.len();
        for (index, (openxr_view, target)) in located.iter().zip(targets).enumerate() {
            let view = Self::view_from_openxr(
                index,
                view_count,
                openxr_view,
                target.width,
                target.height,
            )?;
            {
                let mut eye = context.flows[index].eye_mut();
                if hardware_pose_valid {
                    eye.position = view.position;
                    eye.view = view.view;
                }
                eye.proj = view.projection;
                if let Some(gaze) = gaze_vectors[index] {
                    eye.gaze = gaze;
                }
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

    fn locate_gaze_vectors<G>(
        &self,
        session: &xr::Session<G>,
        eye_tracking: &EyeTracking,
        predicted_display_time: xr::Time,
        base_space: &xr::Space,
        located: &[xr::View],
    ) -> Result<Vec<Option<Vector3<f32>>>, RuntimeError> {
        let mut sample_time = xrsys::EyeGazeSampleTimeEXT::out(ptr::null_mut());
        let mut location =
            xrsys::SpaceLocation::out(sample_time.as_mut_ptr() as *mut xrsys::BaseOutStructure);
        let result = unsafe {
            (session.instance().fp().locate_space)(
                eye_tracking.gaze_space.as_raw(),
                base_space.as_raw(),
                predicted_display_time,
                location.as_mut_ptr(),
            )
        };
        if result != xrsys::Result::SUCCESS {
            return Ok(vec![None; located.len()]);
        }

        let location = unsafe { location.assume_init() };
        let _sample_time = unsafe { sample_time.assume_init() };
        if !location
            .location_flags
            .contains(xrsys::SpaceLocationFlags::ORIENTATION_VALID)
        {
            return Ok(vec![None; located.len()]);
        }

        let gaze_orientation = Quaternion::new(
            location.pose.orientation.w,
            location.pose.orientation.x,
            location.pose.orientation.y,
            location.pose.orientation.z,
        );
        let world_gaze = Matrix3::from(gaze_orientation) * Vector3::unit_z();

        let mut gaze_vectors = Vec::with_capacity(located.len());
        for view in located {
            let view_orientation = Quaternion::new(
                view.pose.orientation.w,
                view.pose.orientation.x,
                view.pose.orientation.y,
                view.pose.orientation.z,
            );
            let eye_gaze = Matrix3::from(view_orientation).invert().ok_or_else(|| {
                RuntimeError::View("OpenXR returned a singular eye pose".to_string())
            })? * world_gaze;
            gaze_vectors.push(Some(if eye_gaze.magnitude2() > 0.0 {
                eye_gaze.normalize()
            } else {
                Vector3::unit_z()
            }));
        }

        Ok(gaze_vectors)
    }

    fn view_from_openxr(
        index: usize,
        view_count: usize,
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
            eye_index: Self::eye_index_for_view_count(view_count, index),
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

    fn enabled_extensions(
        &self,
        backend: Backend,
        available_extensions: &xr::ExtensionSet,
    ) -> xr::ExtensionSet {
        let mut extensions = xr::ExtensionSet::default();

        match backend {
            Backend::Vulkan => extensions.khr_vulkan_enable2 = true,
            #[cfg(target_vendor = "apple")]
            Backend::Metal => extensions.khr_metal_enable = true,
            #[cfg(not(target_vendor = "apple"))]
            Backend::Metal => {}
            Backend::Auto => {}
        }

        #[cfg(target_os = "android")]
        {
            extensions.khr_android_create_instance = true;
        }

        if available_extensions.ext_eye_gaze_interaction {
            extensions.ext_eye_gaze_interaction = true;
        }
        if available_extensions.varjo_quad_views {
            extensions.varjo_quad_views = true;
        }
        if available_extensions.varjo_foveated_rendering && extensions.varjo_quad_views {
            extensions.varjo_foveated_rendering = true;
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
