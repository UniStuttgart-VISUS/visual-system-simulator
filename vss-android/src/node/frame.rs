use std::collections::VecDeque;
use std::ptr::NonNull;
use std::sync::{Arc, Mutex};

use ash::vk;
use log::*;

use jni::objects::JObject;
use jni::{EnvUnowned, Outcome};

use ndk_sys::{
    AHardwareBuffer, AHardwareBuffer_Desc, AHardwareBuffer_acquire, AHardwareBuffer_describe,
    AHardwareBuffer_fromHardwareBuffer, AHardwareBuffer_release,
};

use vss::*;

const ANDROID_HARDWARE_BUFFER_EXTENSION: &std::ffi::CStr =
    ash::vk::ANDROID_EXTERNAL_MEMORY_ANDROID_HARDWARE_BUFFER_NAME;

const AHARDWAREBUFFER_USAGE_GPU_SAMPLED_IMAGE: u64 = 0x100;

pub enum Frame {
    Hardware(HardwareBufferFrame),
    Rgba(RgbBuffer),
}

#[derive(Default)]
pub struct SharedFrame {
    generation: u64,
    frame: Option<Frame>,
}

impl SharedFrame {
    pub fn publish(&mut self, frame: Frame) {
        self.generation = self.generation.wrapping_add(1);
        self.frame = Some(frame);
    }
}

pub struct HardwareBufferFrame {
    buffer: HardwareBufferRef,
    width: u32,
    height: u32,
    data_space: i32,
    rotation_degrees: i32,
}

unsafe impl Send for HardwareBufferFrame {}

impl Clone for HardwareBufferFrame {
    fn clone(&self) -> Self {
        Self {
            buffer: self.buffer.clone(),
            width: self.width,
            height: self.height,
            data_space: self.data_space,
            rotation_degrees: self.rotation_degrees,
        }
    }
}

impl HardwareBufferFrame {
    pub fn from_jni(
        env: &mut EnvUnowned<'_>,
        hardware_buffer: JObject<'_>,
        width: u32,
        height: u32,
        data_space: i32,
        rotation_degrees: i32,
    ) -> Option<Self> {
        let ptr = match env
            .with_env_no_catch(|env| -> jni::errors::Result<_> {
                Ok(unsafe {
                    AHardwareBuffer_fromHardwareBuffer(
                        env.get_raw() as *mut _,
                        hardware_buffer.as_raw() as *mut _,
                    )
                })
            })
            .into_outcome()
        {
            Outcome::Ok(ptr) => ptr,
            Outcome::Err(_) | Outcome::Panic(_) => return None,
        };
        let ptr = NonNull::new(ptr)?;
        Some(Self {
            buffer: unsafe { HardwareBufferRef::new(ptr) },
            width,
            height,
            data_space,
            rotation_degrees: normalize_rotation_degrees(rotation_degrees),
        })
    }

    fn output_size(&self) -> [u32; 2] {
        oriented_size(self.width, self.height, self.rotation_degrees)
    }
}

pub fn oriented_size(width: u32, height: u32, rotation_degrees: i32) -> [u32; 2] {
    match normalize_rotation_degrees(rotation_degrees) {
        90 | 270 => [height.max(1), width.max(1)],
        _ => [width.max(1), height.max(1)],
    }
}

struct HardwareBufferRef {
    ptr: NonNull<AHardwareBuffer>,
}

unsafe impl Send for HardwareBufferRef {}

impl Clone for HardwareBufferRef {
    fn clone(&self) -> Self {
        unsafe { HardwareBufferRef::new(self.ptr) }
    }
}

impl HardwareBufferRef {
    unsafe fn new(ptr: NonNull<AHardwareBuffer>) -> Self {
        unsafe {
            AHardwareBuffer_acquire(ptr.as_ptr());
        }
        Self { ptr }
    }

    fn describe(&self) -> AHardwareBuffer_Desc {
        unsafe {
            let mut desc = std::mem::zeroed();
            AHardwareBuffer_describe(self.ptr.as_ptr(), &mut desc);
            desc
        }
    }

    fn import_hardware_buffer(
        &self,
        context: &RenderContext,
        data_space: i32,
        rotation_degrees: i32,
    ) -> Result<ExternalHardwareBufferTexture, HardwareBufferImportError> {
        let device = context.device();
        let Some(hal_device) = (unsafe { device.as_hal::<wgpu_hal::api::Vulkan>() }) else {
            return Err(HardwareBufferImportError::Unsupported(
                "wgpu device is not Vulkan".to_string(),
            ));
        };

        if !hal_device
            .enabled_device_extensions()
            .contains(&ANDROID_HARDWARE_BUFFER_EXTENSION)
        {
            return Err(HardwareBufferImportError::Unsupported(
                "Vulkan device was created without VK_ANDROID_external_memory_android_hardware_buffer"
                    .to_string(),
            ));
        }

        let desc = self.describe();
        if desc.usage & AHARDWAREBUFFER_USAGE_GPU_SAMPLED_IMAGE == 0 {
            return Err(HardwareBufferImportError::Unsupported(format!(
                "AHardwareBuffer usage {:#x} is missing AHARDWAREBUFFER_USAGE_GPU_SAMPLED_IMAGE",
                desc.usage
            )));
        }

        let ahb = ash::android::external_memory_android_hardware_buffer::Device::new(
            hal_device.shared_instance().raw_instance(),
            hal_device.raw_device(),
        );
        let (
            allocation_size,
            memory_type_bits,
            queried_format,
            external_format,
            ycbcr_model,
            ycbcr_range,
            ycbcr_components,
            x_chroma_offset,
            y_chroma_offset,
        ) = {
            let mut format_properties = vk::AndroidHardwareBufferFormatPropertiesANDROID::default();
            let mut properties = vk::AndroidHardwareBufferPropertiesANDROID::default()
                .push_next(&mut format_properties);
            unsafe {
                ahb.get_android_hardware_buffer_properties(
                    self.ptr.as_ptr() as *const vk::AHardwareBuffer,
                    &mut properties,
                )
                .map_err(|err| HardwareBufferImportError::Vulkan {
                    step: "vkGetAndroidHardwareBufferPropertiesANDROID",
                    err,
                })?;
            }
            (
                properties.allocation_size,
                properties.memory_type_bits,
                format_properties.format,
                format_properties.external_format,
                format_properties.suggested_ycbcr_model,
                format_properties.suggested_ycbcr_range,
                format_properties.sampler_ycbcr_conversion_components,
                format_properties.suggested_x_chroma_offset,
                format_properties.suggested_y_chroma_offset,
            )
        };
        let raw_device = hal_device.raw_device().clone();

        let format = if queried_format != vk::Format::UNDEFINED {
            queried_format
        } else if external_format == 0 {
            vk_format_from_ahb(desc.format).ok_or_else(|| {
                HardwareBufferImportError::Unsupported(format!(
                    "Unsupported RGBA AHardwareBuffer format {}",
                    desc.format
                ))
            })?
        } else {
            vk::Format::UNDEFINED
        };
        let uses_external_format = format == vk::Format::UNDEFINED;

        let conversion = if !uses_external_format {
            None
        } else {
            let mut conversion_external_format =
                vk::ExternalFormatANDROID::default().external_format(external_format);
            let conversion_info = vk::SamplerYcbcrConversionCreateInfo::default()
                .push_next(&mut conversion_external_format)
                .format(format)
                .ycbcr_model(ycbcr_model)
                .ycbcr_range(ycbcr_range)
                .components(ycbcr_components)
                .x_chroma_offset(x_chroma_offset)
                .y_chroma_offset(y_chroma_offset)
                .chroma_filter(vk::Filter::LINEAR)
                .force_explicit_reconstruction(false);
            Some(unsafe {
                raw_device
                    .create_sampler_ycbcr_conversion(&conversion_info, None)
                    .map_err(|err| HardwareBufferImportError::Vulkan {
                        step: "vkCreateSamplerYcbcrConversion",
                        err,
                    })?
            })
        };

        let sampler_info = vk::SamplerCreateInfo::default()
            .mag_filter(vk::Filter::LINEAR)
            .min_filter(vk::Filter::LINEAR)
            .mipmap_mode(vk::SamplerMipmapMode::NEAREST)
            .address_mode_u(vk::SamplerAddressMode::CLAMP_TO_EDGE)
            .address_mode_v(vk::SamplerAddressMode::CLAMP_TO_EDGE)
            .address_mode_w(vk::SamplerAddressMode::CLAMP_TO_EDGE)
            .min_lod(0.0)
            .max_lod(0.0);
        let sampler = unsafe {
            if let Some(conversion) = conversion {
                let mut sampler_conversion =
                    vk::SamplerYcbcrConversionInfo::default().conversion(conversion);
                raw_device.create_sampler(&sampler_info.push_next(&mut sampler_conversion), None)
            } else {
                raw_device.create_sampler(&sampler_info, None)
            }
            .inspect_err(|_| {
                if let Some(conversion) = conversion {
                    raw_device.destroy_sampler_ycbcr_conversion(conversion, None);
                }
            })
            .map_err(|err| HardwareBufferImportError::Vulkan {
                step: "vkCreateSampler",
                err,
            })?
        };

        let mut external_image = vk::ExternalMemoryImageCreateInfo::default()
            .handle_types(vk::ExternalMemoryHandleTypeFlags::ANDROID_HARDWARE_BUFFER_ANDROID);
        let image_info = vk::ImageCreateInfo::default()
            .image_type(vk::ImageType::TYPE_2D)
            .format(format)
            .extent(vk::Extent3D {
                width: desc.width.max(1),
                height: desc.height.max(1),
                depth: 1,
            })
            .mip_levels(1)
            .array_layers(1)
            .samples(vk::SampleCountFlags::TYPE_1)
            .tiling(vk::ImageTiling::OPTIMAL)
            .usage(vk::ImageUsageFlags::SAMPLED)
            .sharing_mode(vk::SharingMode::EXCLUSIVE)
            .initial_layout(vk::ImageLayout::UNDEFINED);
        let image = unsafe {
            let result = if !uses_external_format {
                raw_device.create_image(&image_info.push_next(&mut external_image), None)
            } else {
                let mut image_external_format =
                    vk::ExternalFormatANDROID::default().external_format(external_format);
                raw_device.create_image(
                    &image_info
                        .push_next(&mut image_external_format)
                        .push_next(&mut external_image),
                    None,
                )
            };
            result
                .inspect_err(|_| {
                    raw_device.destroy_sampler(sampler, None);
                    if let Some(conversion) = conversion {
                        raw_device.destroy_sampler_ycbcr_conversion(conversion, None);
                    }
                })
                .map_err(|err| HardwareBufferImportError::Vulkan {
                    step: "vkCreateImage",
                    err,
                })?
        };

        let destroy_image_resources = || unsafe {
            raw_device.destroy_image(image, None);
            raw_device.destroy_sampler(sampler, None);
            if let Some(conversion) = conversion {
                raw_device.destroy_sampler_ycbcr_conversion(conversion, None);
            }
        };

        let memory_type_index = find_memory_type_index(
            hal_device.shared_instance().raw_instance(),
            hal_device.raw_physical_device(),
            memory_type_bits,
            vk::MemoryPropertyFlags::DEVICE_LOCAL,
        )
        .ok_or_else(|| {
            destroy_image_resources();
            HardwareBufferImportError::NoMemoryType
        })?;

        let mut dedicated = vk::MemoryDedicatedAllocateInfo::default().image(image);
        let mut import = vk::ImportAndroidHardwareBufferInfoANDROID::default()
            .buffer(self.ptr.as_ptr() as *mut vk::AHardwareBuffer);
        let allocation_info = vk::MemoryAllocateInfo::default()
            .push_next(&mut dedicated)
            .push_next(&mut import)
            .allocation_size(allocation_size)
            .memory_type_index(memory_type_index);
        let memory = unsafe {
            raw_device
                .allocate_memory(&allocation_info, None)
                .inspect_err(|_| destroy_image_resources())
                .map_err(|err| HardwareBufferImportError::Vulkan {
                    step: "vkAllocateMemory",
                    err,
                })?
        };

        unsafe {
            raw_device
                .bind_image_memory(image, memory, 0)
                .inspect_err(|_| {
                    raw_device.free_memory(memory, None);
                    destroy_image_resources();
                })
                .map_err(|err| HardwareBufferImportError::Vulkan {
                    step: "vkBindImageMemory",
                    err,
                })?;
        }

        let view_info = vk::ImageViewCreateInfo::default()
            .image(image)
            .view_type(vk::ImageViewType::TYPE_2D)
            .format(format)
            .subresource_range(
                vk::ImageSubresourceRange::default()
                    .aspect_mask(vk::ImageAspectFlags::COLOR)
                    .base_mip_level(0)
                    .level_count(1)
                    .base_array_layer(0)
                    .layer_count(1),
            );
        let image_view = unsafe {
            let result = if let Some(conversion) = conversion {
                let mut view_conversion =
                    vk::SamplerYcbcrConversionInfo::default().conversion(conversion);
                raw_device.create_image_view(&view_info.push_next(&mut view_conversion), None)
            } else {
                raw_device.create_image_view(&view_info, None)
            };
            result
                .inspect_err(|_| {
                    raw_device.free_memory(memory, None);
                    destroy_image_resources();
                })
                .map_err(|err| HardwareBufferImportError::Vulkan {
                    step: "vkCreateImageView",
                    err,
                })?
        };

        unsafe {
            AHardwareBuffer_acquire(self.ptr.as_ptr());
        }

        Ok(ExternalHardwareBufferTexture {
            raw_device,
            queue_family_index: hal_device.queue_family_index(),
            image,
            memory,
            image_view,
            sampler,
            conversion,
            hardware_buffer: self.ptr.as_ptr() as usize,
            external_format,
            data_space,
            rotation_degrees,
            ycbcr_model,
            ycbcr_range,
            ycbcr_components,
            x_chroma_offset,
            y_chroma_offset,
            source_image_ready: false,
            render_target_image: None,
            target_image_layout: vk::ImageLayout::UNDEFINED,
            render_resources: None,
        })
    }
}

struct ExternalHardwareBufferTexture {
    raw_device: ash::Device,
    queue_family_index: u32,
    image: vk::Image,
    memory: vk::DeviceMemory,
    image_view: vk::ImageView,
    sampler: vk::Sampler,
    conversion: Option<vk::SamplerYcbcrConversion>,
    hardware_buffer: usize,
    external_format: u64,
    data_space: i32,
    rotation_degrees: i32,
    ycbcr_model: vk::SamplerYcbcrModelConversion,
    ycbcr_range: vk::SamplerYcbcrRange,
    ycbcr_components: vk::ComponentMapping,
    x_chroma_offset: vk::ChromaLocation,
    y_chroma_offset: vk::ChromaLocation,
    source_image_ready: bool,
    render_target_image: Option<vk::Image>,
    target_image_layout: vk::ImageLayout,
    render_resources: Option<ExternalRenderResources>,
}

impl ExternalHardwareBufferTexture {
    fn conversion_metadata(&self) -> String {
        format!(
            "dataspace={:#x}, rotation={}deg, external_format={:#x}, ycbcr_model={:?}, ycbcr_range={:?}, components={:?}, chroma_offset=({:?}, {:?})",
            self.data_space,
            self.rotation_degrees,
            self.external_format,
            self.ycbcr_model,
            self.ycbcr_range,
            self.ycbcr_components,
            self.x_chroma_offset,
            self.y_chroma_offset,
        )
    }

    fn render_to(
        &mut self,
        context: &RenderContext,
        target: &RenderTexture,
        output_format: wgpu::TextureFormat,
    ) -> Result<(), HardwareBufferImportError> {
        let target_texture = target.texture.as_ref().ok_or_else(|| {
            HardwareBufferImportError::Unsupported(
                "Hardware-buffer render needs a texture-backed RenderTexture".to_string(),
            )
        })?;
        let target_view = {
            let Some(target_view) = (unsafe { target.view.as_hal::<wgpu_hal::api::Vulkan>() })
            else {
                return Err(HardwareBufferImportError::Unsupported(
                    "Hardware-buffer render target is not a Vulkan texture view".to_string(),
                ));
            };
            unsafe { target_view.raw_handle() }
        };
        let target_image = {
            let Some(target_texture) =
                (unsafe { target_texture.as_hal::<wgpu_hal::api::Vulkan>() })
            else {
                return Err(HardwareBufferImportError::Unsupported(
                    "Hardware-buffer render target is not a Vulkan texture".to_string(),
                ));
            };
            unsafe { target_texture.raw_handle() }
        };
        if self.render_target_image != Some(target_image) {
            self.render_target_image = Some(target_image);
            self.target_image_layout = vk::ImageLayout::UNDEFINED;
            self.render_resources = None;
        }
        let target_format = vk_format_from_wgpu(output_format).ok_or_else(|| {
            HardwareBufferImportError::Unsupported(format!(
                "Unsupported hardware-buffer output format {output_format:?}"
            ))
        })?;
        let Some(hal_device) = (unsafe { context.device().as_hal::<wgpu_hal::api::Vulkan>() })
        else {
            return Err(HardwareBufferImportError::Unsupported(
                "Hardware-buffer render device is not Vulkan".to_string(),
            ));
        };

        if self.render_resources.is_none() {
            self.render_resources = Some(ExternalRenderResources::new(
                &self.raw_device,
                self.image_view,
                self.sampler,
                target_view,
                target.width,
                target.height,
                target_format,
                self.rotation_degrees,
                hal_device.shared_instance().raw_instance(),
                hal_device.raw_physical_device(),
            )?);
        }
        let resources = self.render_resources.as_ref().unwrap();

        let mut encoder =
            context
                .device()
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Hardware-buffer render encoder"),
                });
        let command_result = unsafe {
            encoder.as_hal_mut::<wgpu_hal::api::Vulkan, _, _>(|hal_encoder| {
                let Some(hal_encoder) = hal_encoder else {
                    return Err(HardwareBufferImportError::Unsupported(
                        "Hardware-buffer render command encoder is not Vulkan".to_string(),
                    ));
                };
                let command_buffer = hal_encoder.raw_handle();
                self.record_target_to_color_barrier(command_buffer, target_image);
                self.record_image_ready_barrier(command_buffer);
                resources.record_draw(
                    &self.raw_device,
                    command_buffer,
                    target.width,
                    target.height,
                );
                self.record_target_to_shader_read_barrier(command_buffer, target_image);
                Ok(())
            })
        };
        command_result?;

        self.source_image_ready = true;
        self.target_image_layout = vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL;

        context.queue().submit(std::iter::once(encoder.finish()));
        Ok(())
    }

    unsafe fn record_image_ready_barrier(&self, command_buffer: vk::CommandBuffer) {
        if self.source_image_ready {
            return;
        }

        let barrier = vk::ImageMemoryBarrier::default()
            .src_access_mask(vk::AccessFlags::empty())
            .dst_access_mask(vk::AccessFlags::SHADER_READ)
            .old_layout(vk::ImageLayout::GENERAL)
            .new_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
            .src_queue_family_index(vk::QUEUE_FAMILY_FOREIGN_EXT)
            .dst_queue_family_index(self.queue_family_index)
            .image(self.image)
            .subresource_range(
                vk::ImageSubresourceRange::default()
                    .aspect_mask(vk::ImageAspectFlags::COLOR)
                    .base_mip_level(0)
                    .level_count(1)
                    .base_array_layer(0)
                    .layer_count(1),
            );
        unsafe {
            self.raw_device.cmd_pipeline_barrier(
                command_buffer,
                vk::PipelineStageFlags::TOP_OF_PIPE,
                vk::PipelineStageFlags::FRAGMENT_SHADER,
                vk::DependencyFlags::empty(),
                &[],
                &[],
                &[barrier],
            );
        }
    }

    unsafe fn record_target_to_color_barrier(
        &self,
        command_buffer: vk::CommandBuffer,
        target_image: vk::Image,
    ) {
        let (src_stage, src_access_mask, old_layout) =
            if self.target_image_layout == vk::ImageLayout::UNDEFINED {
                (
                    vk::PipelineStageFlags::TOP_OF_PIPE,
                    vk::AccessFlags::empty(),
                    vk::ImageLayout::UNDEFINED,
                )
            } else {
                (
                    vk::PipelineStageFlags::FRAGMENT_SHADER,
                    vk::AccessFlags::SHADER_READ,
                    vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
                )
            };
        let barrier = vk::ImageMemoryBarrier::default()
            .src_access_mask(src_access_mask)
            .dst_access_mask(vk::AccessFlags::COLOR_ATTACHMENT_WRITE)
            .old_layout(old_layout)
            .new_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
            .image(target_image)
            .subresource_range(
                vk::ImageSubresourceRange::default()
                    .aspect_mask(vk::ImageAspectFlags::COLOR)
                    .base_mip_level(0)
                    .level_count(1)
                    .base_array_layer(0)
                    .layer_count(1),
            );
        unsafe {
            self.raw_device.cmd_pipeline_barrier(
                command_buffer,
                src_stage,
                vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
                vk::DependencyFlags::empty(),
                &[],
                &[],
                &[barrier],
            );
        }
    }

    unsafe fn record_target_to_shader_read_barrier(
        &self,
        command_buffer: vk::CommandBuffer,
        target_image: vk::Image,
    ) {
        let barrier = vk::ImageMemoryBarrier::default()
            .src_access_mask(vk::AccessFlags::COLOR_ATTACHMENT_WRITE)
            .dst_access_mask(vk::AccessFlags::SHADER_READ)
            .old_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
            .new_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
            .image(target_image)
            .subresource_range(
                vk::ImageSubresourceRange::default()
                    .aspect_mask(vk::ImageAspectFlags::COLOR)
                    .base_mip_level(0)
                    .level_count(1)
                    .base_array_layer(0)
                    .layer_count(1),
            );
        unsafe {
            self.raw_device.cmd_pipeline_barrier(
                command_buffer,
                vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
                vk::PipelineStageFlags::FRAGMENT_SHADER,
                vk::DependencyFlags::empty(),
                &[],
                &[],
                &[barrier],
            );
        }
    }
}

impl Drop for ExternalHardwareBufferTexture {
    fn drop(&mut self) {
        unsafe {
            drop(self.render_resources.take());
            self.raw_device.destroy_image_view(self.image_view, None);
            self.raw_device.destroy_sampler(self.sampler, None);
            if let Some(conversion) = self.conversion {
                self.raw_device
                    .destroy_sampler_ycbcr_conversion(conversion, None);
            }
            self.raw_device.free_memory(self.memory, None);
            self.raw_device.destroy_image(self.image, None);
            AHardwareBuffer_release(self.hardware_buffer as *mut AHardwareBuffer);
        }
    }
}

struct ExternalRenderResources {
    raw_device: ash::Device,
    render_pass: vk::RenderPass,
    framebuffer: vk::Framebuffer,
    rotation_buffer: vk::Buffer,
    rotation_memory: vk::DeviceMemory,
    descriptor_set_layout: vk::DescriptorSetLayout,
    descriptor_pool: vk::DescriptorPool,
    pipeline_layout: vk::PipelineLayout,
    pipeline: vk::Pipeline,
    vertex_shader: vk::ShaderModule,
    fragment_shader: vk::ShaderModule,
    descriptor_set: vk::DescriptorSet,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct RotationUniform {
    rotation_rad: f32,
    _padding: [f32; 3],
}

impl ExternalRenderResources {
    fn new(
        raw_device: &ash::Device,
        image_view: vk::ImageView,
        sampler: vk::Sampler,
        target_view: vk::ImageView,
        width: u32,
        height: u32,
        target_format: vk::Format,
        rotation_degrees: i32,
        instance: &ash::Instance,
        physical_device: vk::PhysicalDevice,
    ) -> Result<Self, HardwareBufferImportError> {
        let raw_device = raw_device.clone();
        let color_attachment = vk::AttachmentDescription::default()
            .format(target_format)
            .samples(vk::SampleCountFlags::TYPE_1)
            .load_op(vk::AttachmentLoadOp::CLEAR)
            .store_op(vk::AttachmentStoreOp::STORE)
            .stencil_load_op(vk::AttachmentLoadOp::DONT_CARE)
            .stencil_store_op(vk::AttachmentStoreOp::DONT_CARE)
            .initial_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
            .final_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL);
        let color_ref = vk::AttachmentReference::default()
            .attachment(0)
            .layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL);
        let subpass = vk::SubpassDescription::default()
            .pipeline_bind_point(vk::PipelineBindPoint::GRAPHICS)
            .color_attachments(std::slice::from_ref(&color_ref));
        let render_pass_info = vk::RenderPassCreateInfo::default()
            .attachments(std::slice::from_ref(&color_attachment))
            .subpasses(std::slice::from_ref(&subpass));
        let render_pass = unsafe {
            raw_device
                .create_render_pass(&render_pass_info, None)
                .map_err(|err| HardwareBufferImportError::Vulkan {
                    step: "vkCreateRenderPass",
                    err,
                })?
        };

        let framebuffer_info = vk::FramebufferCreateInfo::default()
            .render_pass(render_pass)
            .attachments(std::slice::from_ref(&target_view))
            .width(width.max(1))
            .height(height.max(1))
            .layers(1);
        let framebuffer = unsafe {
            raw_device
                .create_framebuffer(&framebuffer_info, None)
                .inspect_err(|_| raw_device.destroy_render_pass(render_pass, None))
                .map_err(|err| HardwareBufferImportError::Vulkan {
                    step: "vkCreateFramebuffer",
                    err,
                })?
        };

        let rotation_uniform = RotationUniform {
            rotation_rad: rotation_radians(rotation_degrees),
            _padding: [0.0; 3],
        };
        let (rotation_buffer, rotation_memory) = create_rotation_uniform_buffer(
            &raw_device,
            instance,
            physical_device,
            rotation_uniform,
        )
        .inspect_err(|_| unsafe {
            raw_device.destroy_framebuffer(framebuffer, None);
            raw_device.destroy_render_pass(render_pass, None);
        })?;

        let immutable_samplers = [sampler];
        let bindings = [
            vk::DescriptorSetLayoutBinding::default()
                .binding(0)
                .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                .descriptor_count(1)
                .stage_flags(vk::ShaderStageFlags::FRAGMENT)
                .immutable_samplers(&immutable_samplers),
            vk::DescriptorSetLayoutBinding::default()
                .binding(2)
                .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
                .descriptor_count(1)
                .stage_flags(vk::ShaderStageFlags::VERTEX),
        ];
        let descriptor_set_layout_info =
            vk::DescriptorSetLayoutCreateInfo::default().bindings(&bindings);
        let descriptor_set_layout = unsafe {
            raw_device
                .create_descriptor_set_layout(&descriptor_set_layout_info, None)
                .inspect_err(|_| {
                    raw_device.destroy_buffer(rotation_buffer, None);
                    raw_device.free_memory(rotation_memory, None);
                    raw_device.destroy_framebuffer(framebuffer, None);
                    raw_device.destroy_render_pass(render_pass, None);
                })
                .map_err(|err| HardwareBufferImportError::Vulkan {
                    step: "vkCreateDescriptorSetLayout",
                    err,
                })?
        };
        let pool_sizes = [
            vk::DescriptorPoolSize::default()
                .ty(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                .descriptor_count(1),
            vk::DescriptorPoolSize::default()
                .ty(vk::DescriptorType::UNIFORM_BUFFER)
                .descriptor_count(1),
        ];
        let descriptor_pool_info = vk::DescriptorPoolCreateInfo::default()
            .max_sets(1)
            .pool_sizes(&pool_sizes);
        let descriptor_pool = unsafe {
            raw_device
                .create_descriptor_pool(&descriptor_pool_info, None)
                .inspect_err(|_| {
                    raw_device.destroy_descriptor_set_layout(descriptor_set_layout, None);
                    raw_device.destroy_buffer(rotation_buffer, None);
                    raw_device.free_memory(rotation_memory, None);
                    raw_device.destroy_framebuffer(framebuffer, None);
                    raw_device.destroy_render_pass(render_pass, None);
                })
                .map_err(|err| HardwareBufferImportError::Vulkan {
                    step: "vkCreateDescriptorPool",
                    err,
                })?
        };
        let set_layouts = [descriptor_set_layout];
        let descriptor_set_info = vk::DescriptorSetAllocateInfo::default()
            .descriptor_pool(descriptor_pool)
            .set_layouts(&set_layouts);
        let descriptor_set = unsafe {
            raw_device
                .allocate_descriptor_sets(&descriptor_set_info)
                .inspect_err(|_| {
                    raw_device.destroy_descriptor_pool(descriptor_pool, None);
                    raw_device.destroy_descriptor_set_layout(descriptor_set_layout, None);
                    raw_device.destroy_buffer(rotation_buffer, None);
                    raw_device.free_memory(rotation_memory, None);
                    raw_device.destroy_framebuffer(framebuffer, None);
                    raw_device.destroy_render_pass(render_pass, None);
                })
                .map_err(|err| HardwareBufferImportError::Vulkan {
                    step: "vkAllocateDescriptorSets",
                    err,
                })?[0]
        };
        let image_info = [vk::DescriptorImageInfo::default()
            .sampler(sampler)
            .image_view(image_view)
            .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)];
        let rotation_info = [vk::DescriptorBufferInfo::default()
            .buffer(rotation_buffer)
            .offset(0)
            .range(std::mem::size_of::<RotationUniform>() as u64)];
        let descriptor_write = [
            vk::WriteDescriptorSet::default()
                .dst_set(descriptor_set)
                .dst_binding(0)
                .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                .image_info(&image_info),
            vk::WriteDescriptorSet::default()
                .dst_set(descriptor_set)
                .dst_binding(2)
                .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
                .buffer_info(&rotation_info),
        ];
        unsafe {
            raw_device.update_descriptor_sets(&descriptor_write, &[]);
        }

        let set_layouts = [descriptor_set_layout];
        let pipeline_layout_info =
            vk::PipelineLayoutCreateInfo::default().set_layouts(&set_layouts);
        let pipeline_layout = unsafe {
            raw_device
                .create_pipeline_layout(&pipeline_layout_info, None)
                .map_err(|err| HardwareBufferImportError::Vulkan {
                    step: "vkCreatePipelineLayout",
                    err,
                })?
        };

        let vertex_shader =
            create_wgsl_shader_module(&raw_device, EXTERNAL_VERTEX_SHADER, "vs_main")?;
        let fragment_shader =
            create_spirv_shader_module(&raw_device, EXTERNAL_FRAGMENT_SHADER_SPIRV)?;
        let vertex_entry = unsafe { std::ffi::CStr::from_bytes_with_nul_unchecked(b"vs_main\0") };
        let fragment_entry = unsafe { std::ffi::CStr::from_bytes_with_nul_unchecked(b"main\0") };
        let shader_stages = [
            vk::PipelineShaderStageCreateInfo::default()
                .stage(vk::ShaderStageFlags::VERTEX)
                .module(vertex_shader)
                .name(vertex_entry),
            vk::PipelineShaderStageCreateInfo::default()
                .stage(vk::ShaderStageFlags::FRAGMENT)
                .module(fragment_shader)
                .name(fragment_entry),
        ];
        let vertex_input = vk::PipelineVertexInputStateCreateInfo::default();
        let input_assembly = vk::PipelineInputAssemblyStateCreateInfo::default()
            .topology(vk::PrimitiveTopology::TRIANGLE_LIST);
        let viewport = vk::Viewport {
            x: 0.0,
            y: height as f32,
            width: width as f32,
            height: -(height as f32),
            min_depth: 0.0,
            max_depth: 1.0,
        };
        let scissor = vk::Rect2D {
            offset: vk::Offset2D { x: 0, y: 0 },
            extent: vk::Extent2D {
                width: width.max(1),
                height: height.max(1),
            },
        };
        let viewport_state = vk::PipelineViewportStateCreateInfo::default()
            .viewports(std::slice::from_ref(&viewport))
            .scissors(std::slice::from_ref(&scissor));
        let rasterization = vk::PipelineRasterizationStateCreateInfo::default()
            .polygon_mode(vk::PolygonMode::FILL)
            .cull_mode(vk::CullModeFlags::NONE)
            .front_face(vk::FrontFace::COUNTER_CLOCKWISE)
            .line_width(1.0);
        let multisample = vk::PipelineMultisampleStateCreateInfo::default()
            .rasterization_samples(vk::SampleCountFlags::TYPE_1);
        let color_blend_attachment = vk::PipelineColorBlendAttachmentState::default()
            .color_write_mask(vk::ColorComponentFlags::RGBA);
        let color_blend = vk::PipelineColorBlendStateCreateInfo::default()
            .attachments(std::slice::from_ref(&color_blend_attachment));
        let pipeline_info = vk::GraphicsPipelineCreateInfo::default()
            .stages(&shader_stages)
            .vertex_input_state(&vertex_input)
            .input_assembly_state(&input_assembly)
            .viewport_state(&viewport_state)
            .rasterization_state(&rasterization)
            .multisample_state(&multisample)
            .color_blend_state(&color_blend)
            .layout(pipeline_layout)
            .render_pass(render_pass)
            .subpass(0);
        let pipeline = unsafe {
            raw_device
                .create_graphics_pipelines(vk::PipelineCache::null(), &[pipeline_info], None)
                .map_err(|(_, err)| HardwareBufferImportError::Vulkan {
                    step: "vkCreateGraphicsPipelines",
                    err,
                })?[0]
        };

        Ok(Self {
            raw_device,
            render_pass,
            framebuffer,
            rotation_buffer,
            rotation_memory,
            descriptor_set_layout,
            descriptor_pool,
            pipeline_layout,
            pipeline,
            vertex_shader,
            fragment_shader,
            descriptor_set,
        })
    }

    unsafe fn record_draw(
        &self,
        raw_device: &ash::Device,
        command_buffer: vk::CommandBuffer,
        width: u32,
        height: u32,
    ) {
        let clear_values = [vk::ClearValue {
            color: vk::ClearColorValue {
                float32: [0.0, 0.0, 0.0, 1.0],
            },
        }];
        let render_area = vk::Rect2D {
            offset: vk::Offset2D { x: 0, y: 0 },
            extent: vk::Extent2D {
                width: width.max(1),
                height: height.max(1),
            },
        };
        let begin_info = vk::RenderPassBeginInfo::default()
            .render_pass(self.render_pass)
            .framebuffer(self.framebuffer)
            .render_area(render_area)
            .clear_values(&clear_values);
        unsafe {
            raw_device.cmd_begin_render_pass(
                command_buffer,
                &begin_info,
                vk::SubpassContents::INLINE,
            );
            raw_device.cmd_bind_pipeline(
                command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                self.pipeline,
            );
            raw_device.cmd_bind_descriptor_sets(
                command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                self.pipeline_layout,
                0,
                &[self.descriptor_set],
                &[],
            );
            raw_device.cmd_draw(command_buffer, 3, 1, 0, 0);
            raw_device.cmd_end_render_pass(command_buffer);
        }
    }
}

impl Drop for ExternalRenderResources {
    fn drop(&mut self) {
        unsafe {
            self.raw_device.destroy_pipeline(self.pipeline, None);
            self.raw_device
                .destroy_pipeline_layout(self.pipeline_layout, None);
            self.raw_device
                .destroy_shader_module(self.fragment_shader, None);
            self.raw_device
                .destroy_shader_module(self.vertex_shader, None);
            self.raw_device
                .destroy_descriptor_pool(self.descriptor_pool, None);
            self.raw_device
                .destroy_descriptor_set_layout(self.descriptor_set_layout, None);
            self.raw_device.destroy_buffer(self.rotation_buffer, None);
            self.raw_device.free_memory(self.rotation_memory, None);
            self.raw_device.destroy_framebuffer(self.framebuffer, None);
            self.raw_device.destroy_render_pass(self.render_pass, None);
        }
    }
}

#[derive(Debug)]
enum HardwareBufferImportError {
    Unsupported(String),
    NoMemoryType,
    Vulkan { step: &'static str, err: vk::Result },
}

impl std::fmt::Display for HardwareBufferImportError {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HardwareBufferImportError::Unsupported(message) => write!(fmt, "{message}"),
            HardwareBufferImportError::NoMemoryType => {
                write!(
                    fmt,
                    "No suitable Vulkan memory type for AHardwareBuffer import"
                )
            }
            HardwareBufferImportError::Vulkan { step, err } => {
                write!(fmt, "{step} failed: {err:?}")
            }
        }
    }
}

impl Drop for HardwareBufferRef {
    fn drop(&mut self) {
        unsafe {
            AHardwareBuffer_release(self.ptr.as_ptr());
        }
    }
}

pub struct FrameNode {
    shared_frame: Arc<Mutex<SharedFrame>>,
    generation: u64,
    targets: ColorDepthTargets,
    output_size: [u32; 2],
    frame_count: u64,
    renderer: HardwareBufferRenderer,
    rgba_renderer: UploadRgbBuffer,
    use_rgba: bool,
}

impl FrameNode {
    pub fn new(context: &RenderContext, shared_frame: Arc<Mutex<SharedFrame>>) -> Self {
        Self {
            shared_frame,
            generation: 0,
            targets: ColorDepthTargets::new(context.device(), "FrameNode"),
            output_size: [1, 1],
            frame_count: 0,
            renderer: HardwareBufferRenderer::new(),
            rgba_renderer: UploadRgbBuffer::new(context),
            use_rgba: false,
        }
    }

    fn receive_latest_frame(&mut self) -> bool {
        let shared = self.shared_frame.lock().unwrap();
        if shared.generation == self.generation {
            return false;
        }
        let Some(frame) = shared.frame.as_ref() else {
            return false;
        };
        self.generation = shared.generation;

        if let Frame::Rgba(buffer) = frame {
            warn!(
                "RGBA image upload path is active ({}x{})",
                buffer.width, buffer.height
            );
            self.output_size = [buffer.width.max(1), buffer.height.max(1)];
            self.rgba_renderer.upload_buffer(&buffer);
            self.use_rgba = true;
            return true;
        }
        let Frame::Hardware(frame) = frame else {
            unreachable!()
        };
        self.use_rgba = false;

        self.frame_count += 1;
        if self.frame_count == 1 || self.frame_count.is_multiple_of(120) {
            let desc = frame.buffer.describe();
            warn!(
                "Received hardware-buffer camera frame {}x{} (AHB {}x{}, format={}, usage={:#x})",
                frame.width, frame.height, desc.width, desc.height, desc.format, desc.usage
            );
        }

        // The import shader rotates the camera image into display orientation.
        // A quarter turn also swaps the logical extent; keeping the sensor
        // extent here would stretch the rotated image into the old aspect ratio.
        self.output_size = frame.output_size();
        self.renderer.queue_frame(frame.clone());
        true
    }
}

impl Node for FrameNode {
    fn name(&self) -> &'static str {
        "FrameNode"
    }

    fn negociate_slots(
        &mut self,
        context: &RenderContext,
        slots: NodeSlots,
        original_image: &mut Option<Texture>,
    ) -> NodeSlots {
        if self.use_rgba {
            return self
                .rgba_renderer
                .negociate_slots(context, slots, original_image);
        }
        let slots = slots.emplace_color_depth_output(
            context,
            self.output_size[0],
            self.output_size[1],
            "FrameNode",
        );
        self.targets = slots.as_color_depth_targets();

        let (color_out, _) = slots.as_color_depth_target();
        original_image.replace(color_out.as_texture());

        slots
    }

    fn input(&mut self, eye: &EyeInput, _mouse: &MouseInput) -> (EyeInput, NodeChanges) {
        let changes = if self.receive_latest_frame() {
            NodeChanges::OUTPUT
        } else {
            NodeChanges::empty()
        };
        (eye.clone(), changes.normalized())
    }

    fn render(
        &mut self,
        context: &RenderContext,
        _encoder: &mut wgpu::CommandEncoder,
        screen: Option<&RenderTexture>,
    ) {
        if self.use_rgba {
            self.rgba_renderer.render(context, _encoder, screen);
            return;
        }
        let target = screen.unwrap_or(&self.targets.rt_color);
        match self
            .renderer
            .render(context, target, context.output_format())
        {
            ZeroCopyRender::Rendered | ZeroCopyRender::NoFrame => {}
            ZeroCopyRender::FrameImportFailed(err) => {
                warn!("Hardware-buffer zero-copy frame import failed: {err}");
            }
            ZeroCopyRender::RenderFailed(err) => {
                warn!("Hardware-buffer zero-copy render failed: {err}");
            }
        }
    }
}

struct HardwareBufferRenderer {
    pending_frame: Option<HardwareBufferFrame>,
    current_texture: Option<ExternalHardwareBufferTexture>,
    retired_textures: VecDeque<ExternalHardwareBufferTexture>,
    logged_active: bool,
}

impl HardwareBufferRenderer {
    fn new() -> Self {
        Self {
            pending_frame: None,
            current_texture: None,
            retired_textures: VecDeque::new(),
            logged_active: false,
        }
    }

    fn queue_frame(&mut self, frame: HardwareBufferFrame) {
        self.pending_frame = Some(frame);
    }

    fn render(
        &mut self,
        context: &RenderContext,
        target: &RenderTexture,
        output_format: wgpu::TextureFormat,
    ) -> ZeroCopyRender {
        if let Some(frame) = self.pending_frame.take() {
            match frame.buffer.import_hardware_buffer(
                context,
                frame.data_space,
                frame.rotation_degrees,
            ) {
                Ok(texture) => {
                    if !self.logged_active {
                        warn!(
                            "HardwareBuffer zero-copy path is active ({})",
                            texture.conversion_metadata()
                        );
                        self.logged_active = true;
                    }
                    if let Some(old_texture) = self.current_texture.replace(texture) {
                        self.retired_textures.push_back(old_texture);
                    }
                    while self.retired_textures.len() > 4 {
                        self.retired_textures.pop_front();
                    }
                }
                Err(err) if self.current_texture.is_some() => {
                    warn!(
                        "Hardware-buffer zero-copy frame import failed, reusing previous external frame: {err}"
                    );
                }
                Err(err) => return ZeroCopyRender::FrameImportFailed(err),
            }
        }

        let Some(texture) = self.current_texture.as_mut() else {
            return ZeroCopyRender::NoFrame;
        };

        match texture.render_to(context, target, output_format) {
            Ok(()) => ZeroCopyRender::Rendered,
            Err(err) => ZeroCopyRender::RenderFailed(err),
        }
    }
}

enum ZeroCopyRender {
    Rendered,
    NoFrame,
    FrameImportFailed(HardwareBufferImportError),
    RenderFailed(HardwareBufferImportError),
}

fn vk_format_from_wgpu(format: wgpu::TextureFormat) -> Option<vk::Format> {
    match format {
        wgpu::TextureFormat::Rgba8Unorm => Some(vk::Format::R8G8B8A8_UNORM),
        wgpu::TextureFormat::Rgba8UnormSrgb => Some(vk::Format::R8G8B8A8_SRGB),
        wgpu::TextureFormat::Bgra8Unorm => Some(vk::Format::B8G8R8A8_UNORM),
        wgpu::TextureFormat::Bgra8UnormSrgb => Some(vk::Format::B8G8R8A8_SRGB),
        _ => None,
    }
}

fn vk_format_from_ahb(format: u32) -> Option<vk::Format> {
    match format {
        1 => Some(vk::Format::R8G8B8A8_UNORM),
        2 => Some(vk::Format::R8G8B8A8_UNORM),
        3 => Some(vk::Format::R8G8B8_UNORM),
        4 => Some(vk::Format::R5G6B5_UNORM_PACK16),
        22 => Some(vk::Format::R16G16B16A16_SFLOAT),
        43 => Some(vk::Format::A2B10G10R10_UNORM_PACK32),
        _ => None,
    }
}

fn normalize_rotation_degrees(rotation_degrees: i32) -> i32 {
    let normalized = rotation_degrees.rem_euclid(360);
    match normalized {
        45..=134 => 90,
        135..=224 => 180,
        225..=314 => 270,
        _ => 0,
    }
}

fn rotation_radians(rotation_degrees: i32) -> f32 {
    (normalize_rotation_degrees(rotation_degrees) as f32).to_radians()
}

fn create_rotation_uniform_buffer(
    raw_device: &ash::Device,
    instance: &ash::Instance,
    physical_device: vk::PhysicalDevice,
    uniform: RotationUniform,
) -> Result<(vk::Buffer, vk::DeviceMemory), HardwareBufferImportError> {
    let buffer_info = vk::BufferCreateInfo::default()
        .size(std::mem::size_of::<RotationUniform>() as u64)
        .usage(vk::BufferUsageFlags::UNIFORM_BUFFER)
        .sharing_mode(vk::SharingMode::EXCLUSIVE);
    let buffer = unsafe {
        raw_device
            .create_buffer(&buffer_info, None)
            .map_err(|err| HardwareBufferImportError::Vulkan {
                step: "vkCreateBuffer",
                err,
            })?
    };

    let memory_requirements = unsafe { raw_device.get_buffer_memory_requirements(buffer) };
    let memory_type_index = find_memory_type_index(
        instance,
        physical_device,
        memory_requirements.memory_type_bits,
        vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
    )
    .ok_or_else(|| {
        unsafe {
            raw_device.destroy_buffer(buffer, None);
        }
        HardwareBufferImportError::NoMemoryType
    })?;

    let allocation_info = vk::MemoryAllocateInfo::default()
        .allocation_size(memory_requirements.size)
        .memory_type_index(memory_type_index);
    let memory = unsafe {
        raw_device
            .allocate_memory(&allocation_info, None)
            .inspect_err(|_| raw_device.destroy_buffer(buffer, None))
            .map_err(|err| HardwareBufferImportError::Vulkan {
                step: "vkAllocateMemory",
                err,
            })?
    };

    unsafe {
        raw_device
            .bind_buffer_memory(buffer, memory, 0)
            .inspect_err(|_| {
                raw_device.free_memory(memory, None);
                raw_device.destroy_buffer(buffer, None);
            })
            .map_err(|err| HardwareBufferImportError::Vulkan {
                step: "vkBindBufferMemory",
                err,
            })?;

        let mapped = raw_device
            .map_memory(
                memory,
                0,
                std::mem::size_of::<RotationUniform>() as u64,
                vk::MemoryMapFlags::empty(),
            )
            .inspect_err(|_| {
                raw_device.free_memory(memory, None);
                raw_device.destroy_buffer(buffer, None);
            })
            .map_err(|err| HardwareBufferImportError::Vulkan {
                step: "vkMapMemory",
                err,
            })?;
        std::ptr::copy_nonoverlapping(
            &uniform as *const RotationUniform as *const u8,
            mapped as *mut u8,
            std::mem::size_of::<RotationUniform>(),
        );
        raw_device.unmap_memory(memory);
    }

    Ok((buffer, memory))
}

fn create_wgsl_shader_module(
    raw_device: &ash::Device,
    source: &str,
    entry_point: &str,
) -> Result<vk::ShaderModule, HardwareBufferImportError> {
    let spirv = compile_wgsl_to_spirv(source, entry_point).map_err(|err| {
        HardwareBufferImportError::Unsupported(format!(
            "Cannot compile hardware-buffer shader: {err}"
        ))
    })?;
    let shader_info = vk::ShaderModuleCreateInfo::default().code(&spirv);
    unsafe {
        raw_device
            .create_shader_module(&shader_info, None)
            .map_err(|err| HardwareBufferImportError::Vulkan {
                step: "vkCreateShaderModule",
                err,
            })
    }
}

fn create_spirv_shader_module(
    raw_device: &ash::Device,
    spirv: &[u32],
) -> Result<vk::ShaderModule, HardwareBufferImportError> {
    let shader_info = vk::ShaderModuleCreateInfo::default().code(spirv);
    unsafe {
        raw_device
            .create_shader_module(&shader_info, None)
            .map_err(|err| HardwareBufferImportError::Vulkan {
                step: "vkCreateShaderModule",
                err,
            })
    }
}

fn compile_wgsl_to_spirv(source: &str, entry_point: &str) -> Result<Vec<u32>, String> {
    let mut frontend = naga::front::wgsl::Frontend::new();
    let module = frontend.parse(source).map_err(|err| err.to_string())?;
    let mut validator = naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::all(),
    );
    let info = validator.validate(&module).map_err(|err| err.to_string())?;
    let stage = module
        .entry_points
        .iter()
        .find(|entry| entry.name == entry_point)
        .map(|entry| entry.stage)
        .ok_or_else(|| format!("entry point {entry_point} not found"))?;
    let options = naga::back::spv::Options::default();
    let pipeline_options = naga::back::spv::PipelineOptions {
        shader_stage: stage,
        entry_point: entry_point.to_string(),
    };
    naga::back::spv::write_vec(&module, &info, &options, Some(&pipeline_options))
        .map_err(|err| err.to_string())
}

fn find_memory_type_index(
    instance: &ash::Instance,
    physical_device: vk::PhysicalDevice,
    type_bits: u32,
    required_flags: vk::MemoryPropertyFlags,
) -> Option<u32> {
    let properties = unsafe { instance.get_physical_device_memory_properties(physical_device) };
    properties
        .memory_types_as_slice()
        .iter()
        .enumerate()
        .find_map(|(index, memory_type)| {
            let supported = type_bits & (1 << index) != 0;
            let has_flags = memory_type.property_flags & required_flags == required_flags;
            (supported && has_flags).then_some(index as u32)
        })
}

const EXTERNAL_VERTEX_SHADER: &str = r#"
struct RotationUniform {
    rotation_rad: f32,
};

@group(0) @binding(2) var<uniform> rotation: RotationUniform;

struct VertexOut {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOut {
    let positions = array(vec2<f32>(-1.0, -1.0), vec2<f32>(3.0, -1.0), vec2<f32>(-1.0, 3.0));
    let pos = positions[vertex_index];
    let centered = pos * 0.5;

    let c = cos(rotation.rotation_rad);
    let s = sin(rotation.rotation_rad);
    let rotated = vec2(c * centered.x + s * centered.y, -s * centered.x + c * centered.y);

    var out: VertexOut;
    out.position = vec4(pos, 0.0, 1.0);
    out.uv = rotated + vec2(0.5, 0.5);
    return out;
}
"#;

// WORKAROUND for gfx-rs/wgpu#4386 (`texture_external` support).
// As of 2026-07-09, Naga 29 can parse WGSL `texture_external`, but its SPIR-V
// backend does not emit `ImageClass::External`. This path needs a Vulkan
// combined image sampler for Android external-format YCbCr sampling, so keep the
// fragment shader as precompiled SPIR-V from GLSL for now.
//
// GLSL source:
// #version 450
// layout(set = 0, binding = 0) uniform sampler2D hardware_buffer_texture;
// layout(location = 0) in vec2 uv;
// layout(location = 0) out vec4 out_color;
// void main() { out_color = texture(hardware_buffer_texture, uv); }
const EXTERNAL_FRAGMENT_SHADER_SPIRV: &[u32] = &[
    0x07230203, 0x00010000, 0x00000000, 0x00000013, 0x00000000, 0x00020011, 0x00000001, 0x0003000e,
    0x00000000, 0x00000001, 0x0007000f, 0x00000004, 0x00000001, 0x6e69616d, 0x00000000, 0x0000000c,
    0x0000000e, 0x00030010, 0x00000001, 0x00000007, 0x00040047, 0x0000000a, 0x00000022, 0x00000000,
    0x00040047, 0x0000000a, 0x00000021, 0x00000000, 0x00040047, 0x0000000c, 0x0000001e, 0x00000000,
    0x00040047, 0x0000000e, 0x0000001e, 0x00000000, 0x00020013, 0x00000002, 0x00030021, 0x00000003,
    0x00000002, 0x00030016, 0x00000004, 0x00000020, 0x00040017, 0x00000005, 0x00000004, 0x00000002,
    0x00040017, 0x00000006, 0x00000004, 0x00000004, 0x00090019, 0x00000007, 0x00000004, 0x00000001,
    0x00000000, 0x00000000, 0x00000000, 0x00000001, 0x00000000, 0x0003001b, 0x00000008, 0x00000007,
    0x00040020, 0x00000009, 0x00000000, 0x00000008, 0x0004003b, 0x00000009, 0x0000000a, 0x00000000,
    0x00040020, 0x0000000b, 0x00000001, 0x00000005, 0x0004003b, 0x0000000b, 0x0000000c, 0x00000001,
    0x00040020, 0x0000000d, 0x00000003, 0x00000006, 0x0004003b, 0x0000000d, 0x0000000e, 0x00000003,
    0x00050036, 0x00000002, 0x00000001, 0x00000000, 0x00000003, 0x000200f8, 0x0000000f, 0x0004003d,
    0x00000008, 0x00000010, 0x0000000a, 0x0004003d, 0x00000005, 0x00000011, 0x0000000c, 0x00050057,
    0x00000006, 0x00000012, 0x00000010, 0x00000011, 0x0003003e, 0x0000000e, 0x00000012, 0x000100fd,
    0x00010038,
];
