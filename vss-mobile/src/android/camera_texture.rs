use std::collections::VecDeque;
use std::ptr::NonNull;
use std::sync::mpsc::Receiver;

use ash::vk;
use log::*;

use jni::objects::JObject;
use jni::{EnvUnowned, Outcome};

use ndk_sys::{
    AHardwareBuffer, AHardwareBuffer_Desc, AHardwareBuffer_Plane, AHardwareBuffer_Planes,
    AHardwareBuffer_UsageFlags, AHardwareBuffer_acquire, AHardwareBuffer_describe,
    AHardwareBuffer_fromHardwareBuffer, AHardwareBuffer_lockPlanes, AHardwareBuffer_release,
    AHardwareBuffer_unlock, ARect,
};

use vss::*;

#[allow(dead_code)]
const ANDROID_HARDWARE_BUFFER_EXTENSION: &std::ffi::CStr =
    ash::vk::ANDROID_EXTERNAL_MEMORY_ANDROID_HARDWARE_BUFFER_NAME;

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

    fn import_external_ycbcr(
        &self,
        context: &RenderContext,
        width: u32,
        height: u32,
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

        let ahb = ash::android::external_memory_android_hardware_buffer::Device::new(
            hal_device.shared_instance().raw_instance(),
            hal_device.raw_device(),
        );
        let (
            allocation_size,
            memory_type_bits,
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
                format_properties.external_format,
                format_properties.suggested_ycbcr_model,
                format_properties.suggested_ycbcr_range,
                format_properties.sampler_ycbcr_conversion_components,
                format_properties.suggested_x_chroma_offset,
                format_properties.suggested_y_chroma_offset,
            )
        };

        if external_format == 0 {
            return Err(HardwareBufferImportError::Unsupported(
                "AHardwareBuffer did not report an Android external format".to_string(),
            ));
        }

        let mut conversion_external_format =
            vk::ExternalFormatANDROID::default().external_format(external_format);
        let conversion_info = vk::SamplerYcbcrConversionCreateInfo::default()
            .push_next(&mut conversion_external_format)
            .format(vk::Format::UNDEFINED)
            .ycbcr_model(ycbcr_model)
            .ycbcr_range(ycbcr_range)
            .components(ycbcr_components)
            .x_chroma_offset(x_chroma_offset)
            .y_chroma_offset(y_chroma_offset)
            .chroma_filter(vk::Filter::LINEAR)
            .force_explicit_reconstruction(false);
        let raw_device = hal_device.raw_device().clone();
        let conversion = unsafe {
            raw_device
                .create_sampler_ycbcr_conversion(&conversion_info, None)
                .map_err(|err| HardwareBufferImportError::Vulkan {
                    step: "vkCreateSamplerYcbcrConversion",
                    err,
                })?
        };

        let mut sampler_conversion =
            vk::SamplerYcbcrConversionInfo::default().conversion(conversion);
        let sampler_info = vk::SamplerCreateInfo::default()
            .push_next(&mut sampler_conversion)
            .mag_filter(vk::Filter::LINEAR)
            .min_filter(vk::Filter::LINEAR)
            .mipmap_mode(vk::SamplerMipmapMode::NEAREST)
            .address_mode_u(vk::SamplerAddressMode::CLAMP_TO_EDGE)
            .address_mode_v(vk::SamplerAddressMode::CLAMP_TO_EDGE)
            .address_mode_w(vk::SamplerAddressMode::CLAMP_TO_EDGE)
            .min_lod(0.0)
            .max_lod(0.0);
        let sampler = unsafe {
            raw_device
                .create_sampler(&sampler_info, None)
                .inspect_err(|_| raw_device.destroy_sampler_ycbcr_conversion(conversion, None))
                .map_err(|err| HardwareBufferImportError::Vulkan {
                    step: "vkCreateSampler",
                    err,
                })?
        };

        let mut image_external_format =
            vk::ExternalFormatANDROID::default().external_format(external_format);
        let mut external_image = vk::ExternalMemoryImageCreateInfo::default()
            .handle_types(vk::ExternalMemoryHandleTypeFlags::ANDROID_HARDWARE_BUFFER_ANDROID);
        let image_info = vk::ImageCreateInfo::default()
            .push_next(&mut image_external_format)
            .push_next(&mut external_image)
            .image_type(vk::ImageType::TYPE_2D)
            .format(vk::Format::UNDEFINED)
            .extent(vk::Extent3D {
                width: width.max(1),
                height: height.max(1),
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
            raw_device
                .create_image(&image_info, None)
                .inspect_err(|_| {
                    raw_device.destroy_sampler(sampler, None);
                    raw_device.destroy_sampler_ycbcr_conversion(conversion, None);
                })
                .map_err(|err| HardwareBufferImportError::Vulkan {
                    step: "vkCreateImage",
                    err,
                })?
        };

        let memory_requirements = unsafe { raw_device.get_image_memory_requirements(image) };
        let memory_type_index = find_memory_type_index(
            hal_device.shared_instance().raw_instance(),
            hal_device.raw_physical_device(),
            memory_type_bits & memory_requirements.memory_type_bits,
            vk::MemoryPropertyFlags::DEVICE_LOCAL,
        )
        .ok_or_else(|| {
            unsafe {
                raw_device.destroy_image(image, None);
                raw_device.destroy_sampler(sampler, None);
                raw_device.destroy_sampler_ycbcr_conversion(conversion, None);
            }
            HardwareBufferImportError::NoMemoryType
        })?;

        let mut dedicated = vk::MemoryDedicatedAllocateInfo::default().image(image);
        let mut import = vk::ImportAndroidHardwareBufferInfoANDROID::default()
            .buffer(self.ptr.as_ptr() as *mut vk::AHardwareBuffer);
        let allocation_info = vk::MemoryAllocateInfo::default()
            .push_next(&mut dedicated)
            .push_next(&mut import)
            .allocation_size(allocation_size.max(memory_requirements.size))
            .memory_type_index(memory_type_index);
        let memory = unsafe {
            raw_device
                .allocate_memory(&allocation_info, None)
                .inspect_err(|_| {
                    raw_device.destroy_image(image, None);
                    raw_device.destroy_sampler(sampler, None);
                    raw_device.destroy_sampler_ycbcr_conversion(conversion, None);
                })
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
                    raw_device.destroy_image(image, None);
                    raw_device.destroy_sampler(sampler, None);
                    raw_device.destroy_sampler_ycbcr_conversion(conversion, None);
                })
                .map_err(|err| HardwareBufferImportError::Vulkan {
                    step: "vkBindImageMemory",
                    err,
                })?;
        }

        let mut view_conversion = vk::SamplerYcbcrConversionInfo::default().conversion(conversion);
        let view_info = vk::ImageViewCreateInfo::default()
            .push_next(&mut view_conversion)
            .image(image)
            .view_type(vk::ImageViewType::TYPE_2D)
            .format(vk::Format::UNDEFINED)
            .subresource_range(
                vk::ImageSubresourceRange::default()
                    .aspect_mask(vk::ImageAspectFlags::COLOR)
                    .base_mip_level(0)
                    .level_count(1)
                    .base_array_layer(0)
                    .layer_count(1),
            );
        let image_view = unsafe {
            raw_device
                .create_image_view(&view_info, None)
                .inspect_err(|_| {
                    raw_device.free_memory(memory, None);
                    raw_device.destroy_image(image, None);
                    raw_device.destroy_sampler(sampler, None);
                    raw_device.destroy_sampler_ycbcr_conversion(conversion, None);
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
            render_resources: None,
        })
    }

    fn lock_yuv(&self, width: u32, height: u32) -> Result<YuvBuffer, String> {
        let rect = ARect {
            left: 0,
            top: 0,
            right: width as i32,
            bottom: height as i32,
        };
        let mut planes: AHardwareBuffer_Planes = unsafe { std::mem::zeroed() };
        let usage = AHardwareBuffer_UsageFlags::AHARDWAREBUFFER_USAGE_CPU_READ_RARELY.0 as u64;
        let result =
            unsafe { AHardwareBuffer_lockPlanes(self.ptr.as_ptr(), usage, -1, &rect, &mut planes) };
        if result != 0 {
            return Err(format!(
                "AHardwareBuffer_lockPlanes failed with status {result}"
            ));
        }

        let yuv = (|| {
            if planes.planeCount < 3 {
                return Err(format!(
                    "Expected at least 3 YUV planes, got {}",
                    planes.planeCount
                ));
            }
            let pixels_y = copy_plane(&planes.planes[0], width, height)?;
            let pixels_u = copy_chroma_plane(&planes.planes[1], width, height)?;
            let pixels_v = copy_chroma_plane(&planes.planes[2], width, height)?;
            Ok(YuvBuffer {
                pixels_y,
                pixels_u,
                pixels_v,
                width,
                height,
            })
        })();

        unsafe {
            AHardwareBuffer_unlock(self.ptr.as_ptr(), std::ptr::null_mut());
        }
        yuv
    }
}

struct ExternalHardwareBufferTexture {
    raw_device: ash::Device,
    image: vk::Image,
    memory: vk::DeviceMemory,
    image_view: vk::ImageView,
    sampler: vk::Sampler,
    conversion: vk::SamplerYcbcrConversion,
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
                "External YCbCr render needs a texture-backed RenderTexture".to_string(),
            )
        })?;
        let target_view = {
            let Some(target_view) = (unsafe { target.view.as_hal::<wgpu_hal::api::Vulkan>() })
            else {
                return Err(HardwareBufferImportError::Unsupported(
                    "External YCbCr render target is not a Vulkan texture view".to_string(),
                ));
            };
            unsafe { target_view.raw_handle() }
        };
        let target_image = {
            let Some(target_texture) =
                (unsafe { target_texture.as_hal::<wgpu_hal::api::Vulkan>() })
            else {
                return Err(HardwareBufferImportError::Unsupported(
                    "External YCbCr render target is not a Vulkan texture".to_string(),
                ));
            };
            unsafe { target_texture.raw_handle() }
        };
        let target_format = vk_format_from_wgpu(output_format).ok_or_else(|| {
            HardwareBufferImportError::Unsupported(format!(
                "Unsupported external YCbCr output format {output_format:?}"
            ))
        })?;
        let Some(hal_device) = (unsafe { context.device().as_hal::<wgpu_hal::api::Vulkan>() })
        else {
            return Err(HardwareBufferImportError::Unsupported(
                "External YCbCr render device is not Vulkan".to_string(),
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
        let transition_source_image = !self.source_image_ready;

        let mut encoder =
            context
                .device()
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("External YCbCr render encoder"),
                });
        let command_result = unsafe {
            encoder.as_hal_mut::<wgpu_hal::api::Vulkan, _, _>(|hal_encoder| {
                let Some(hal_encoder) = hal_encoder else {
                    return Err(HardwareBufferImportError::Unsupported(
                        "External YCbCr render command encoder is not Vulkan".to_string(),
                    ));
                };
                let command_buffer = hal_encoder.raw_handle();
                self.record_target_to_color_barrier(command_buffer, target_image);
                if transition_source_image {
                    self.record_image_ready_barrier(command_buffer);
                }
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
        if transition_source_image {
            self.source_image_ready = true;
        }
        context.queue().submit(std::iter::once(encoder.finish()));
        Ok(())
    }

    unsafe fn record_image_ready_barrier(&self, command_buffer: vk::CommandBuffer) {
        let barrier = vk::ImageMemoryBarrier::default()
            .src_access_mask(vk::AccessFlags::empty())
            .dst_access_mask(vk::AccessFlags::SHADER_READ)
            .old_layout(vk::ImageLayout::UNDEFINED)
            .new_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
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
        let barrier = vk::ImageMemoryBarrier::default()
            .src_access_mask(vk::AccessFlags::SHADER_READ)
            .dst_access_mask(vk::AccessFlags::COLOR_ATTACHMENT_WRITE)
            .old_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
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
                vk::PipelineStageFlags::FRAGMENT_SHADER,
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
            self.raw_device
                .destroy_sampler_ycbcr_conversion(self.conversion, None);
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

pub struct CameraTextureNode {
    hardware_frame_receiver: Receiver<HardwareBufferFrame>,
    manual_frame_receiver: Receiver<YuvBuffer>,
    upload: UploadYuvBuffer,
    frame_count: u64,
    backend: CameraTextureBackend,
}

impl CameraTextureNode {
    pub fn new(
        context: &RenderContext,
        hardware_frame_receiver: Receiver<HardwareBufferFrame>,
        manual_frame_receiver: Receiver<YuvBuffer>,
    ) -> Self {
        let mut upload = UploadYuvBuffer::new(context);
        upload.set_format(YuvFormat::YCbCr);
        Self {
            hardware_frame_receiver,
            manual_frame_receiver,
            upload,
            frame_count: 0,
            backend: CameraTextureBackend::probing(),
        }
    }

    fn receive_latest_manual_frame(&mut self) -> bool {
        let mut latest = None;
        while let Ok(frame) = self.manual_frame_receiver.try_recv() {
            latest = Some(frame);
        }

        let Some(frame) = latest else {
            return false;
        };

        debug!(
            "Uploading manual {}x{}px frame...",
            frame.width, frame.height
        );
        self.upload.upload_buffer(frame);
        true
    }

    fn receive_latest_frame(&mut self) -> bool {
        let mut latest = None;
        while let Ok(frame) = self.hardware_frame_receiver.try_recv() {
            latest = Some(frame);
        }

        let Some(frame) = latest else {
            return false;
        };

        self.frame_count += 1;
        if self.frame_count == 1 || self.frame_count.is_multiple_of(120) {
            let desc = frame.buffer.describe();
            warn!(
                "Received hardware-buffer camera frame {}x{} (AHB {}x{}, format={}, usage={:#x})",
                frame.width, frame.height, desc.width, desc.height, desc.format, desc.usage
            );
        }

        self.backend.queue_frame(frame, &mut self.upload)
    }
}

impl Node for CameraTextureNode {
    fn name(&self) -> &'static str {
        "CameraTextureNode"
    }

    fn negociate_slots(
        &mut self,
        context: &RenderContext,
        slots: NodeSlots,
        original_image: &mut Option<Texture>,
    ) -> NodeSlots {
        Node::negociate_slots(&mut self.upload, context, slots, original_image)
    }

    fn input(&mut self, eye: &EyeInput, _mouse: &MouseInput) -> (EyeInput, NodeChanges) {
        let hardware_changes = self.receive_latest_frame();
        let manual_changes = self.receive_latest_manual_frame();
        let mut changes = if hardware_changes || manual_changes {
            NodeChanges::OUTPUT
        } else {
            NodeChanges::empty()
        };
        let (eye, input_changes) = Node::input(&mut self.upload, eye, _mouse);
        changes |= input_changes;
        (eye, changes.normalized())
    }

    fn render(
        &mut self,
        context: &RenderContext,
        encoder: &mut wgpu::CommandEncoder,
        screen: Option<&RenderTexture>,
    ) {
        if self.backend.render(context, &mut self.upload) {
            return;
        }
        Node::render(&mut self.upload, context, encoder, screen);
    }

    fn post_render(&mut self, context: &RenderContext) {
        Node::post_render(&mut self.upload, context);
    }
}

enum CameraTextureBackend {
    Probing {
        zero_copy: ZeroCopyCamera,
        cpu_copy: CpuCopyCamera,
    },
    ZeroCopy(ZeroCopyCamera),
    CpuCopy(CpuCopyCamera),
}

impl CameraTextureBackend {
    fn probing() -> Self {
        Self::Probing {
            zero_copy: ZeroCopyCamera::new(),
            cpu_copy: CpuCopyCamera::new(),
        }
    }

    fn queue_frame(&mut self, frame: HardwareBufferFrame, upload: &mut UploadYuvBuffer) -> bool {
        match self {
            CameraTextureBackend::Probing { zero_copy, .. } => {
                upload.set_output_size(frame.width, frame.height);
                zero_copy.queue_frame(frame);
                true
            }
            CameraTextureBackend::ZeroCopy(zero_copy) => {
                upload.set_output_size(frame.width, frame.height);
                zero_copy.queue_frame(frame);
                true
            }
            CameraTextureBackend::CpuCopy(cpu_copy) => cpu_copy.upload_frame(frame, upload),
        }
    }

    fn render(&mut self, context: &RenderContext, upload: &mut UploadYuvBuffer) -> bool {
        let backend = std::mem::replace(self, CameraTextureBackend::probing());
        match backend {
            CameraTextureBackend::Probing {
                mut zero_copy,
                mut cpu_copy,
            } => match zero_copy.render(context, upload) {
                ZeroCopyRender::Rendered => {
                    *self = CameraTextureBackend::ZeroCopy(zero_copy);
                    true
                }
                ZeroCopyRender::FrameImportFailed { frame, err } => {
                    warn!("Hardware-buffer zero-copy unavailable, switching to CPU copy: {err}");
                    cpu_copy.upload_frame(frame, upload);
                    *self = CameraTextureBackend::CpuCopy(cpu_copy);
                    false
                }
                ZeroCopyRender::RenderFailed(err) => {
                    warn!(
                        "Hardware-buffer zero-copy render unavailable, switching to CPU copy: {err}"
                    );
                    *self = CameraTextureBackend::CpuCopy(cpu_copy);
                    false
                }
                ZeroCopyRender::NoFrame => {
                    *self = CameraTextureBackend::Probing {
                        zero_copy,
                        cpu_copy,
                    };
                    false
                }
            },
            CameraTextureBackend::ZeroCopy(mut zero_copy) => {
                match zero_copy.render(context, upload) {
                    ZeroCopyRender::Rendered => {
                        *self = CameraTextureBackend::ZeroCopy(zero_copy);
                        true
                    }
                    ZeroCopyRender::FrameImportFailed { frame, err } => {
                        warn!(
                            "Hardware-buffer zero-copy unavailable, switching to CPU copy: {err}"
                        );
                        let mut cpu_copy = CpuCopyCamera::new();
                        cpu_copy.upload_frame(frame, upload);
                        *self = CameraTextureBackend::CpuCopy(cpu_copy);
                        false
                    }
                    ZeroCopyRender::RenderFailed(err) => {
                        warn!(
                            "Hardware-buffer zero-copy render unavailable, switching to CPU copy: {err}"
                        );
                        *self = CameraTextureBackend::CpuCopy(CpuCopyCamera::new());
                        false
                    }
                    ZeroCopyRender::NoFrame => {
                        *self = CameraTextureBackend::ZeroCopy(zero_copy);
                        false
                    }
                }
            }
            CameraTextureBackend::CpuCopy(cpu_copy) => {
                *self = CameraTextureBackend::CpuCopy(cpu_copy);
                false
            }
        }
    }
}

struct ZeroCopyCamera {
    pending_frame: Option<HardwareBufferFrame>,
    current_texture: Option<ExternalHardwareBufferTexture>,
    retired_textures: VecDeque<ExternalHardwareBufferTexture>,
    logged_active: bool,
}

impl ZeroCopyCamera {
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

    fn render(&mut self, context: &RenderContext, upload: &UploadYuvBuffer) -> ZeroCopyRender {
        if let Some(frame) = self.pending_frame.take() {
            match frame.buffer.import_external_ycbcr(
                context,
                frame.width.max(1),
                frame.height.max(1),
                frame.data_space,
                frame.rotation_degrees,
            ) {
                Ok(texture) => {
                    if !self.logged_active {
                        warn!(
                            "Camera hardware-buffer zero-copy path is active ({})",
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
                Err(err) => return ZeroCopyRender::FrameImportFailed { frame, err },
            }
        }

        let Some(texture) = self.current_texture.as_mut() else {
            return ZeroCopyRender::NoFrame;
        };

        let target = upload.color_target();
        match texture.render_to(context, &target, context.output_format()) {
            Ok(()) => ZeroCopyRender::Rendered,
            Err(err) => ZeroCopyRender::RenderFailed(err),
        }
    }
}

enum ZeroCopyRender {
    Rendered,
    NoFrame,
    FrameImportFailed {
        frame: HardwareBufferFrame,
        err: HardwareBufferImportError,
    },
    RenderFailed(HardwareBufferImportError),
}

struct CpuCopyCamera {
    logged_active: bool,
}

impl CpuCopyCamera {
    fn new() -> Self {
        Self {
            logged_active: false,
        }
    }

    fn upload_frame(&mut self, frame: HardwareBufferFrame, upload: &mut UploadYuvBuffer) -> bool {
        match frame
            .buffer
            .lock_yuv(frame.width.max(1), frame.height.max(1))
        {
            Ok(buffer) => {
                if !self.logged_active {
                    warn!("Camera hardware-buffer CPU copy path is active");
                    self.logged_active = true;
                }
                upload.upload_buffer(buffer);
                true
            }
            Err(err) => {
                warn!("Hardware-buffer CPU copy failed: {err}");
                false
            }
        }
    }
}

fn copy_plane(plane: &AHardwareBuffer_Plane, width: u32, height: u32) -> Result<Box<[u8]>, String> {
    let data = plane_data(plane)?;
    let width = width as usize;
    let height = height as usize;
    let row_stride = plane.rowStride as usize;
    let pixel_stride = plane.pixelStride.max(1) as usize;
    let mut pixels = vec![0; width * height];

    for row in 0..height {
        let row_base = row * row_stride;
        for col in 0..width {
            pixels[row * width + col] = unsafe { *data.add(row_base + col * pixel_stride) };
        }
    }

    Ok(pixels.into_boxed_slice())
}

fn copy_chroma_plane(
    plane: &AHardwareBuffer_Plane,
    width: u32,
    height: u32,
) -> Result<Box<[u8]>, String> {
    let data = plane_data(plane)?;
    let width = width as usize;
    let height = (height / 2) as usize;
    let row_stride = plane.rowStride as usize;
    let pixel_stride = plane.pixelStride.max(1) as usize;
    let mut pixels = vec![0; width * height / 2];

    for row in 0..height {
        let row_base = row * row_stride;
        for col in 0..(width / 2) {
            pixels[row * (width / 2) + col] = unsafe { *data.add(row_base + col * pixel_stride) };
        }
    }

    Ok(pixels.into_boxed_slice())
}

fn plane_data(plane: &AHardwareBuffer_Plane) -> Result<*const u8, String> {
    NonNull::new(plane.data as *mut u8)
        .map(|ptr| ptr.as_ptr() as *const u8)
        .ok_or_else(|| "AHardwareBuffer plane data was null".to_string())
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
            "Cannot compile external YCbCr shader: {err}"
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
// layout(set = 0, binding = 0) uniform sampler2D camera_texture;
// layout(location = 0) in vec2 uv;
// layout(location = 0) out vec4 out_color;
// void main() { out_color = texture(camera_texture, uv); }
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
