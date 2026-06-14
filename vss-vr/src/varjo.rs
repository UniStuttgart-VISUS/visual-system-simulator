use ash::vk::{self, Handle};
use log::LevelFilter;

use std::{
    ffi::CStr,
    os::raw::{c_char, c_void},
    rc::Rc,
};
use wgpu_hal::InstanceFlags;

use cgmath::{Matrix4, Vector3};
use vss::*;

type VarjoPtr = *mut c_void;

#[repr(C)]
#[derive(Clone)]
struct VulkanData {
    pub instance: vk::Instance,
    pub device: vk::Device,
    pub queue_family_index: u32,
    pub queue_index: u32,
}

#[repr(C)]
#[derive(Clone)]
struct VarjoRenderTarget {
    pub color_image: vk::Image,
    pub depth_image: vk::Image,
    pub width: u32,
    pub height: u32,
}

#[repr(C)]
#[derive(Clone)]
pub struct VarjoViewport {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

#[repr(C)]
#[derive(Clone, Debug)]
pub struct VarjoGazeData {
    pub left_eye: [f32; 3],
    pub right_eye: [f32; 3],
    pub focus_distance: f32,
}

extern "C" {
    fn varjo_new(varjo: *mut VarjoPtr, vulkan_data: *mut VulkanData) -> *const c_char;
    fn varjo_viewports(
        varjo: VarjoPtr,
        viewports: *mut *mut VarjoViewport,
        view_count: *mut i32,
        texture_width: *mut i32,
        texture_height: *mut i32,
    ) -> *const c_char;
    fn varjo_render_targets(
        varjo: VarjoPtr,
        render_targets: *mut *mut VarjoRenderTarget,
        render_targets_size: *mut u32,
    ) -> *const c_char;
    fn varjo_begin_frame_sync(varjo: VarjoPtr, is_available: *mut bool) -> *const c_char;
    fn varjo_current_swap_chain_index(
        varjo: VarjoPtr,
        current_swap_chain_index: *mut u32,
    ) -> *const c_char;
    fn varjo_current_view_matrices(
        varjo: VarjoPtr,
        view_matrix_values: *mut *mut f32,
        view_matrix_count: *mut u32,
    ) -> *const c_char;
    fn varjo_current_proj_matrices(
        varjo: VarjoPtr,
        proj_matrix_values: *mut *mut f32,
        proj_matrix_count: *mut u32,
    ) -> *const c_char;
    fn varjo_current_gaze_data(
        varjo: VarjoPtr,
        is_valid: *mut bool,
        varjo_gaze_data: *mut VarjoGazeData,
    ) -> *const c_char;
    fn varjo_end_frame(varjo: VarjoPtr) -> *const c_char;
    fn varjo_drop(varjo: *mut VarjoPtr);
}

#[derive(Debug)]
pub struct VarjoErr(String);

fn try_fail(error: *const c_char) -> Result<(), VarjoErr> {
    if error == std::ptr::null_mut() {
        Ok(())
    } else {
        let c_str: &CStr = unsafe { CStr::from_ptr(error) };
        let str_slice: &str = c_str.to_str().unwrap();

        Err(VarjoErr(str_slice.to_owned()))
    }
}

fn cstr_from_bytes_until_nul(bytes: &[std::os::raw::c_char]) -> Option<&std::ffi::CStr> {
    if bytes.contains(&0) {
        // Safety for `CStr::from_ptr`:
        // - We've ensured that the slice does contain a null terminator.
        // - The range is valid to read, because the slice covers it.
        // - The memory won't be changed, because the slice borrows it.
        unsafe { Some(std::ffi::CStr::from_ptr(bytes.as_ptr())) }
    } else {
        None
    }
}

pub struct Varjo {
    varjo: VarjoPtr,
    render_targets_color: Vec<RenderTexture>,
    render_targets_depth: Vec<RenderTexture>,
    latest_swap_chain_index: usize,
    pub logging_enabled: bool,
    vulkan_data: VulkanData,
    //pub instance: Option<wgpu::Instance>,
}

impl Varjo {
    pub fn new() -> Self {
        simple_logging::log_to_file("vss_latest.log", LevelFilter::Info).unwrap();

        let mut vulkan_data = VulkanData {
            instance: vk::Instance::from_raw(0),
            device: vk::Device::from_raw(0),
            queue_family_index: 0,
            queue_index: 0,
        };

        println!(
            "rust-side(before): instance: {:?}, device: {:?}",
            vulkan_data.instance, vulkan_data.device
        );

        let mut varjo = std::ptr::null_mut();
        try_fail(unsafe { varjo_new(&mut varjo as *mut *mut _, &mut vulkan_data as *mut _) })
            .unwrap();

        println!(
            "rust-side(after): instance: {:?}, device: {:?}",
            vulkan_data.instance, vulkan_data.device
        );

        Self {
            varjo,
            render_targets_color: Vec::new(),
            render_targets_depth: Vec::new(),
            logging_enabled: true,
            vulkan_data,
            latest_swap_chain_index: 0,
        }
    }

    pub fn check_handles(surface: &Surface) {
        let vulkan_data = unsafe {
            surface.device().as_hal::<wgpu_hal::vulkan::Api, _, _>(
                |vk_device: Option<&wgpu_hal::vulkan::Device>| -> VulkanData {
                    let dev = vk_device.unwrap();
                    VulkanData {
                        instance: dev.shared_instance().raw_instance().handle(),
                        device: dev.raw_device().handle(),
                        queue_family_index: dev.queue_family_index(),
                        queue_index: dev.queue_index(),
                    }
                },
            )
        };
        println!(
            "rust-side(surface): instance: {:?}, device: {:?}",
            vulkan_data.instance, vulkan_data.device
        );
    }

    pub fn create_custom_vk_instance(&self) -> Option<wgpu::Instance> {
        let flags = InstanceFlags::empty();

        let entry = match unsafe { ash::Entry::load() } {
            Ok(entry) => entry,
            Err(err) => {
                log::info!("Missing Vulkan entry points: {:?}", err);
                return None;
            }
        };
        let driver_api_version = match entry.try_enumerate_instance_version() {
            // Vulkan 1.1+
            Ok(Some(version)) => version,
            Ok(None) => vk::API_VERSION_1_0,
            Err(err) => {
                log::warn!("try_enumerate_instance_version: {:?}", err);
                return None;
            }
        };

        /*let app_name = CString::new(desc.name).unwrap();
        let app_info = vk::ApplicationInfo::builder()
            .application_name(app_name.as_c_str())
            .application_version(1)
            .engine_name(CStr::from_bytes_with_nul(b"wgpu-hal\0").unwrap())
            .engine_version(2)
            .api_version(
                // Vulkan 1.0 doesn't like anything but 1.0 passed in here...
                if driver_api_version < vk::API_VERSION_1_1 {
                    vk::API_VERSION_1_0
                } else {
                    // This is the max Vulkan API version supported by `wgpu-hal`.
                    //
                    // If we want to increment this, there are some things that must be done first:
                    //  - Audit the behavioral differences between the previous and new API versions.
                    //  - Audit all extensions used by this backend:
                    //    - If any were promoted in the new API version and the behavior has changed, we must handle the new behavior in addition to the old behavior.
                    //    - If any were obsoleted in the new API version, we must implement a fallback for the new API version
                    //    - If any are non-KHR-vendored, we must ensure the new behavior is still correct (since backwards-compatibility is not guaranteed).
                    vk::HEADER_VERSION_COMPLETE
                },
            );*/

        let extensions =
            match wgpu_hal::vulkan::Instance::desired_extensions(&entry, driver_api_version, flags)
            {
                Ok(extensions) => extensions,
                Err(err) => {
                    log::warn!("desired_extensions: {:?}", err);
                    return None;
                }
            };

        let instance_layers = match unsafe { entry.enumerate_instance_layer_properties() } {
            Ok(layers) => layers,
            Err(err) => {
                log::info!("enumerate_instance_layer_properties: {:?}", err);
                return None;
            }
        };

        let nv_optimus_layer = CStr::from_bytes_with_nul(b"VK_LAYER_NV_optimus\0").unwrap();
        let has_nv_optimus = instance_layers.iter().any(|inst_layer| {
            cstr_from_bytes_until_nul(&inst_layer.layer_name) == Some(nv_optimus_layer)
        });

        // Check requested layers against the available layers
        /*let layers = {
            let mut layers: Vec<&'static CStr> = Vec::new();
            if desc.flags.contains(wgpu_hal::InstanceFlags::VALIDATION) {
                layers.push(CStr::from_bytes_with_nul(b"VK_LAYER_KHRONOS_validation\0").unwrap());
            }

            // Only keep available layers.
            layers.retain(|&layer| {
                if instance_layers.iter().any(|inst_layer| {
                    cstr_from_bytes_until_nul(&inst_layer.layer_name) == Some(layer)
                }) {
                    true
                } else {
                    log::warn!("Unable to find layer: {}", layer.to_string_lossy());
                    false
                }
            });
            layers
        };

        let vk_instance = {
            let str_pointers = layers
                .iter()
                .chain(extensions.iter())
                .map(|&s| {
                    // Safe because `layers` and `extensions` entries have static lifetime.
                    s.as_ptr()
                })
                .collect::<Vec<_>>();

            let create_info = vk::InstanceCreateInfo::builder()
                .flags(vk::InstanceCreateFlags::empty())
                .application_info(&app_info)
                .enabled_layer_names(&str_pointers[..layers.len()])
                .enabled_extension_names(&str_pointers[layers.len()..]);

            unsafe { entry.create_instance(&create_info, None) }.map_err(|e| {
                log::warn!("create_instance: {:?}", e);
                wgpu_hal::InstanceError
            })
        }.unwrap();*/

        let vk_instance =
            unsafe { ash::Instance::load(entry.static_fn(), self.vulkan_data.instance) };

        let wgpu_vk_instance = unsafe {
            wgpu_hal::vulkan::Instance::from_raw(
                entry,
                vk_instance,
                driver_api_version,
                0,
                None,
                extensions,
                flags,
                has_nv_optimus,
                Some(Box::new(|| {})),
            )
        }
        .unwrap();

        Some(unsafe { wgpu::Instance::from_hal::<wgpu_hal::api::Vulkan>(wgpu_vk_instance) })
    }

    pub fn create_custom_vk_device(
        &self,
        instance: &wgpu::Instance,
        adapter: &wgpu::Adapter,
    ) -> (wgpu::Device, wgpu::Queue) {
        let hal_device = unsafe {
            adapter.as_hal::<wgpu_hal::vulkan::Api, _, _>(
                |vk_adapter: Option<&wgpu_hal::vulkan::Adapter>| -> wgpu_hal::OpenDevice<wgpu_hal::vulkan::Api> {
                    let adapter = vk_adapter.unwrap();
                    let features = wgpu::Features::empty();
                    let enabled_extensions = adapter.required_device_extensions(features);

                    println!("features: {:?}", features);

                    for e in &enabled_extensions{
                        println!("enabled extensions: {:?}", e);
                    }

                    let family_index = self.vulkan_data.queue_family_index;
                    let raw_device = ash::Device::load(
                        instance.as_hal::<wgpu_hal::vulkan::Api>().unwrap().shared_instance().raw_instance().fp_v1_0(),
                        self.vulkan_data.device
                    );

                    adapter.device_from_raw(
                        raw_device,
                        Some(Box::new(|| {})),
                        &enabled_extensions,
                        features,
                        &wgpu::MemoryHints::Performance,
                        family_index,
                        self.vulkan_data.queue_index,
                    ).unwrap()
                },
            )
        };

        unsafe {
            adapter.create_device_from_hal(
                hal_device,
                &wgpu::DeviceDescriptor {
                    label: None,
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::default(),
                    memory_hints: wgpu::MemoryHints::Performance,
                    trace: wgpu::Trace::Off,
                },
            )
        }
        .unwrap()
    }

    pub fn get_viewports(&self) -> (Vec<VarjoViewport>, i32, i32) {
        let mut viewports = std::ptr::null_mut::<VarjoViewport>();
        let mut view_count = 0;
        let mut texture_width = 0;
        let mut texture_height = 0;
        try_fail(unsafe {
            varjo_viewports(
                self.varjo,
                &mut viewports as *mut *mut _,
                &mut view_count as *mut _,
                &mut texture_width as *mut _,
                &mut texture_height as *mut _,
            )
        })
        .unwrap();
        println!("varjo_viewports done");
        let viewports = unsafe { std::slice::from_raw_parts(viewports, view_count as usize) };
        (viewports.to_vec(), texture_width, texture_height)
    }

    pub fn create_render_targets(&mut self, surface: &Surface) {
        let mut render_targets = std::ptr::null_mut::<VarjoRenderTarget>();
        let mut render_targets_size = 0u32;
        try_fail(unsafe {
            varjo_render_targets(
                self.varjo,
                &mut render_targets as *mut *mut _,
                &mut render_targets_size as *mut _,
            )
        })
        .unwrap();
        println!("varjo_render_targets done");
        let render_targets =
            unsafe { std::slice::from_raw_parts(render_targets, render_targets_size as usize) };

        let device = surface.device();
        for render_target in render_targets {
            println!("create_render_texture_from_hal");
            let color_texture = create_render_texture_from_hal(
                &device,
                render_target.color_image,
                render_target.width,
                render_target.height,
                wgpu::TextureFormat::Bgra8Unorm,
                create_sampler_nearest(&device),
                Some("Varjo RenderTexture Color"),
            );
            let depth_texture = create_render_texture_from_hal(
                &device,
                render_target.depth_image,
                render_target.width,
                render_target.height,
                wgpu::TextureFormat::Depth24PlusStencil8,
                create_sampler_nearest(&device),
                Some("Varjo RenderTexture Depth"),
            );
            self.render_targets_color.push(color_texture);
            self.render_targets_depth.push(depth_texture);
        }
    }

    pub fn get_current_render_target(&mut self) -> (RenderTexture, RenderTexture) {
        let mut current_swap_chain_index = 0u32;
        try_fail(unsafe {
            varjo_current_swap_chain_index(self.varjo, &mut current_swap_chain_index as *mut _)
        })
        .unwrap();
        self.latest_swap_chain_index = current_swap_chain_index as usize;

        self.get_latest_render_target()
    }

    pub fn get_latest_render_target(&self) -> (RenderTexture, RenderTexture) {
        return (
            self.render_targets_color[self.latest_swap_chain_index].clone(),
            self.render_targets_depth[self.latest_swap_chain_index].clone(),
        );
    }

    pub fn get_current_view_matrices(&self) -> Vec<Matrix4<f32>> {
        let mut view_matrix_values = std::ptr::null_mut::<f32>();
        let mut view_matrix_count = 0u32;
        try_fail(unsafe {
            varjo_current_view_matrices(
                self.varjo,
                &mut view_matrix_values as *mut *mut _,
                &mut view_matrix_count as *mut _,
            )
        })
        .unwrap();
        let matrix_values = unsafe {
            std::slice::from_raw_parts(view_matrix_values, (view_matrix_count * 16) as usize)
        };

        let mut matrices = Vec::new();

        for i in 0..view_matrix_count {
            let m = Matrix4::new(
                matrix_values[(i * 16 + 0) as usize],
                matrix_values[(i * 16 + 1) as usize],
                matrix_values[(i * 16 + 2) as usize],
                matrix_values[(i * 16 + 3) as usize],
                matrix_values[(i * 16 + 4) as usize],
                matrix_values[(i * 16 + 5) as usize],
                matrix_values[(i * 16 + 6) as usize],
                matrix_values[(i * 16 + 7) as usize],
                matrix_values[(i * 16 + 8) as usize],
                matrix_values[(i * 16 + 9) as usize],
                matrix_values[(i * 16 + 10) as usize],
                matrix_values[(i * 16 + 11) as usize],
                matrix_values[(i * 16 + 12) as usize],
                matrix_values[(i * 16 + 13) as usize],
                matrix_values[(i * 16 + 14) as usize],
                matrix_values[(i * 16 + 15) as usize],
            );
            matrices.push(m);
        }
        if self.logging_enabled {
            log::info!("View Matrices {:?}", matrices);
        }
        matrices
    }

    pub fn get_current_proj_matrices(&self) -> Vec<Matrix4<f32>> {
        let mut proj_matrix_values = std::ptr::null_mut::<f32>();
        let mut proj_matrix_count = 0u32;
        try_fail(unsafe {
            varjo_current_proj_matrices(
                self.varjo,
                &mut proj_matrix_values as *mut *mut _,
                &mut proj_matrix_count as *mut _,
            )
        })
        .unwrap();
        let matrix_values = unsafe {
            std::slice::from_raw_parts(proj_matrix_values, (proj_matrix_count * 16) as usize)
        };

        let mut matrices = Vec::new();

        for i in 0..proj_matrix_count {
            let m = Matrix4::new(
                matrix_values[(i * 16 + 0) as usize],
                matrix_values[(i * 16 + 1) as usize],
                matrix_values[(i * 16 + 2) as usize],
                matrix_values[(i * 16 + 3) as usize],
                matrix_values[(i * 16 + 4) as usize],
                matrix_values[(i * 16 + 5) as usize],
                matrix_values[(i * 16 + 6) as usize],
                matrix_values[(i * 16 + 7) as usize],
                matrix_values[(i * 16 + 8) as usize],
                matrix_values[(i * 16 + 9) as usize],
                matrix_values[(i * 16 + 10) as usize],
                matrix_values[(i * 16 + 11) as usize],
                matrix_values[(i * 16 + 12) as usize],
                matrix_values[(i * 16 + 13) as usize],
                matrix_values[(i * 16 + 14) as usize],
                matrix_values[(i * 16 + 15) as usize],
            );
            matrices.push(m);
        }
        if self.logging_enabled {
            log::info!("Proj Matrices {:?}", matrices);
        }
        matrices
    }

    pub fn get_current_gaze(&self) -> (Vector3<f32>, Vector3<f32>, f32) {
        let mut varjo_gaze_data = VarjoGazeData {
            left_eye: [0.0; 3],
            right_eye: [0.0; 3],
            focus_distance: 0.0,
        };
        let mut is_valid = false;
        try_fail(unsafe {
            varjo_current_gaze_data(
                self.varjo,
                &mut is_valid as *mut _,
                &mut varjo_gaze_data as *mut _,
            )
        })
        .unwrap();

        if self.logging_enabled {
            log::info!("{:?}", varjo_gaze_data);
        }

        (
            Vector3::from(varjo_gaze_data.left_eye),
            Vector3::from(varjo_gaze_data.right_eye),
            varjo_gaze_data.focus_distance,
        )
    }

    pub fn begin_frame_sync(&self) -> bool {
        let mut is_available = false;
        try_fail(unsafe { varjo_begin_frame_sync(self.varjo, &mut is_available as *mut _) })
            .unwrap();

        is_available
    }

    pub fn end_frame(&self) {
        try_fail(unsafe { varjo_end_frame(self.varjo) }).unwrap();
    }
}

impl Drop for Varjo {
    fn drop(&mut self) {
        unsafe {
            varjo_drop(&mut self.varjo as *mut *mut _);
        }
    }
}

pub fn create_render_texture_from_hal(
    device: &wgpu::Device,
    raw_image: vk::Image,
    width: u32,
    height: u32,
    format: wgpu::TextureFormat,
    sampler: Sampler,
    label: Option<&str>,
) -> RenderTexture {
    let size = wgpu::Extent3d {
        width,
        height,
        depth_or_array_layers: 1,
    };

    println!("vk::Image: {:?}", raw_image.as_raw());

    println!("create_render_texture_from_hal - create hal_texture");
    let hal_usage = if format.is_depth_stencil_format() {
        wgpu::TextureUses::DEPTH_STENCIL_ATTACHMENT
    } else {
        wgpu::TextureUses::COLOR_TARGET
    };
    let hal_texture = unsafe {
        wgpu_hal::vulkan::Device::texture_from_raw(
            raw_image,
            &wgpu_hal::TextureDescriptor {
                label,
                size,
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format,
                usage: hal_usage,
                memory_flags: wgpu_hal::MemoryFlags::empty(),
                view_formats: vec![format],
            },
            Some(Box::new(|| {})),
        )
    };

    println!("create_render_texture_from_hal - create wgpu texture");
    let texture = unsafe {
        device.create_texture_from_hal::<wgpu_hal::vulkan::Api>(
            hal_texture,
            &wgpu::TextureDescriptor {
                label,
                size,
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                    | wgpu::TextureUsages::TEXTURE_BINDING
                    | wgpu::TextureUsages::COPY_DST,
                view_formats: &[format],
            },
        )
    };

    println!("create_render_texture_from_hal - create view");
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

    RenderTexture {
        texture: Some(Rc::new(texture)),
        view: Rc::new(view),
        sampler: Rc::new(sampler),
        view_dimension: wgpu::TextureViewDimension::D2,
        width,
        height,
        label: label.unwrap_or("Unlabeled").to_string(),
    }
}
