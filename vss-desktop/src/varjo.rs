use ash::vk;
use log::LevelFilter;
use std::{
    num::NonZeroU32,
    os::raw::{c_char, c_void},
    rc::Rc,
};

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
    // pub color_texture_id: u32,
    // pub depth_texture_id: u32,
    pub width: u32,
    pub height: u32,
}

#[repr(C)]
#[derive(Clone)]
pub struct varjo_Viewport {
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
    fn varjo_render_targets(
        varjo: VarjoPtr,
        render_targets: *mut *mut VarjoRenderTarget,
        viewports: *mut *mut varjo_Viewport,
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
        use std::ffi::CStr;
        let c_str: &CStr = unsafe { CStr::from_ptr(error) };
        let str_slice: &str = c_str.to_str().unwrap();

        Err(VarjoErr(str_slice.to_owned()))
    }
}

pub struct Varjo {
    varjo: VarjoPtr,
    render_targets_color: Vec<RenderTexture>,
    render_targets_depth: Vec<RenderTexture>,
    pub logging_enabled: bool,
}

impl Varjo {
    pub fn new(surface: &Surface) -> Self {
        simple_logging::log_to_file("vss_latest.log", LevelFilter::Info).unwrap();

        let mut vulkan_data = unsafe {
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
            "rust-side: instance: {:?}, device: {:?}",
            vulkan_data.instance, vulkan_data.device
        );
        let mut varjo = std::ptr::null_mut();
        try_fail(unsafe { varjo_new(&mut varjo as *mut *mut _, &mut vulkan_data) }).unwrap();
        Self {
            varjo,
            render_targets_color: Vec::new(),
            render_targets_depth: Vec::new(),
            logging_enabled: true,
        }
    }

    pub fn create_render_targets(&mut self, surface: &Surface) -> Vec<varjo_Viewport> {
        let mut render_targets = std::ptr::null_mut::<VarjoRenderTarget>();
        let mut viewports = std::ptr::null_mut::<varjo_Viewport>();
        let mut render_targets_size = 0u32;
        try_fail(unsafe {
            varjo_render_targets(
                self.varjo,
                &mut render_targets as *mut *mut _,
                &mut viewports as *mut *mut _,
                &mut render_targets_size as *mut _,
            )
        })
        .unwrap();
        let render_targets =
            unsafe { std::slice::from_raw_parts(render_targets, render_targets_size as usize) };
        let viewports =
            unsafe { std::slice::from_raw_parts(viewports, render_targets_size as usize) };

        //let mut textures = Vec::new();
        //let mut depth_textures = Vec::new();
        let mut device = surface.device();
        for render_target in render_targets {
            let size = wgpu::Extent3d {
                width: render_target.width,
                height: render_target.height,
                depth_or_array_layers: 1,
            };
            let (color_texture, depth_texture) = unsafe {
                (
                    create_render_texture_from_hal(
                        &device,
                        render_target.color_image,
                        // NonZeroU32::new(render_target.color_texture_id).unwrap(),
                        render_target.width,
                        render_target.height,
                        wgpu::TextureFormat::Rgba8Unorm,
                        create_sampler_nearest(&device),
                        Some("Varjo RenderTexture Color"), // usage: wgpu::TextureUsages::TEXTURE_BINDING |
                                                           //         wgpu::TextureUsages::RENDER_ATTACHMENT |
                                                           //         wgpu::TextureUsages::COPY_SRC,
                    ),
                    create_render_texture_from_hal(
                        &device,
                        render_target.depth_image,
                        // NonZeroU32::new(render_target.depth_texture_id).unwrap(),
                        render_target.width,
                        render_target.height,
                        wgpu::TextureFormat::Depth24PlusStencil8,
                        create_sampler_nearest(&device),
                        Some("Varjo RenderTexture Depth"), // usage: wgpu::TextureUsages::TEXTURE_BINDING |
                                                           //         wgpu::TextureUsages::RENDER_ATTACHMENT |
                                                           //         wgpu::TextureUsages::COPY_SRC,
                    ),
                )
            };
            self.render_targets_color.push(color_texture);
            self.render_targets_depth.push(depth_texture);
        }
        viewports.to_vec()
    }

    pub fn get_current_render_target(&self) -> (RenderTexture, RenderTexture) {
        let mut current_swap_chain_index = 0u32;
        try_fail(unsafe {
            varjo_current_swap_chain_index(self.varjo, &mut current_swap_chain_index as *mut _)
        })
        .unwrap();

        return (
            self.render_targets_color[current_swap_chain_index as usize].clone(),
            self.render_targets_depth[current_swap_chain_index as usize].clone(),
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
    // raw_image: std::num::NonZeroU32,
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

    let hal_texture = unsafe {
        wgpu_hal::vulkan::Device::texture_from_raw(
            raw_image,
            &wgpu_hal::TextureDescriptor {
                label,
                size: size,
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: format,
                usage: wgpu_hal::TextureUses::COLOR_TARGET,
                memory_flags: wgpu_hal::MemoryFlags::TRANSIENT,
                view_formats: vec![format],
            },
            None,
        )
    };

    let texture = unsafe {
        device.create_texture_from_hal::<wgpu_hal::vulkan::Api>(
            hal_texture,
            // device.create_texture_from_hal::<wgpu_hal::gles::Api>(hal_texture,
            &wgpu::TextureDescriptor {
                label,
                size,
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: format,
                usage: wgpu::TextureUsages::TEXTURE_BINDING
                    | wgpu::TextureUsages::RENDER_ATTACHMENT
                    | wgpu::TextureUsages::COPY_SRC, // TODO double check these to lign up with the ones above
                view_formats: &[format],
            },
        )
    };

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
