use std::ffi::c_void;
use std::sync::Mutex;

use objc2::rc::Retained;
use objc2::runtime::ProtocolObject;
use objc2_metal::{MTLTexture, MTLTextureType};
use vss::*;

unsafe extern "C" {
    fn objc_retain(value: *mut c_void) -> *mut c_void;
}

pub struct CameraFrame {
    luma: *mut c_void,
    chroma: *mut c_void,
    depth: Option<*mut c_void>,
    width: u32,
    height: u32,
    depth_width: u32,
    depth_height: u32,
    rotation: i32,
    full_range: bool,
}
unsafe impl Send for CameraFrame {}

impl CameraFrame {
    pub unsafe fn new(
        luma: *mut c_void,
        chroma: *mut c_void,
        depth: *mut c_void,
        width: u32,
        height: u32,
        depth_width: u32,
        depth_height: u32,
        rotation: i32,
        full_range: bool,
    ) -> Option<Self> {
        if luma.is_null() || chroma.is_null() || width == 0 || height == 0 {
            return None;
        }
        let depth = if depth.is_null() || depth_width == 0 || depth_height == 0 {
            None
        } else {
            Some(objc_retain(depth))
        };
        Some(Self {
            luma: objc_retain(luma),
            chroma: objc_retain(chroma),
            depth,
            width,
            height,
            depth_width,
            depth_height,
            rotation: normalize(rotation),
            full_range,
        })
    }
    pub fn output_size(&self) -> [u32; 2] {
        if matches!(self.rotation, 90 | 270) {
            [self.height, self.width]
        } else {
            [self.width, self.height]
        }
    }
}
impl Drop for CameraFrame {
    fn drop(&mut self) {
        unsafe {
            drop(Retained::<ProtocolObject<dyn MTLTexture>>::from_raw(
                self.luma.cast(),
            ));
            drop(Retained::<ProtocolObject<dyn MTLTexture>>::from_raw(
                self.chroma.cast(),
            ));
            if let Some(depth) = self.depth {
                drop(Retained::<ProtocolObject<dyn MTLTexture>>::from_raw(
                    depth.cast(),
                ));
            }
        }
    }
}

pub struct FrameNode {
    pending: &'static Mutex<Option<CameraFrame>>,
    current: Option<ImportedFrame>,
    targets: ColorDepthTargets,
    output_size: [u32; 2],
    pipeline: wgpu::RenderPipeline,
    layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    fallback_depth: wgpu::Texture,
}

impl FrameNode {
    pub fn new(context: &RenderContext, pending: &'static Mutex<Option<CameraFrame>>) -> Self {
        let device = context.device();
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("iOS camera planes"),
            entries: &[
                plane_entry(0),
                plane_entry(1),
                plane_entry(2),
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 4,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("iOS bi-planar YCbCr"),
            source: wgpu::ShaderSource::Wgsl(include_str!("camera.wgsl").into()),
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("iOS camera pipeline"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let mut depth_state = simple_depth_state(DEPTH_FORMAT);
        if let Some(state) = &mut depth_state {
            state.depth_compare = Some(wgpu::CompareFunction::Always);
        }
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("iOS camera conversion"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(context.output_format().into())],
                compilation_options: Default::default(),
            }),
            primitive: Default::default(),
            depth_stencil: depth_state,
            multisample: Default::default(),
            multiview_mask: None,
            cache: None,
        });
        use wgpu::util::DeviceExt;
        let fallback_depth = device.create_texture_with_data(
            context.queue(),
            &wgpu::TextureDescriptor {
                label: Some("iOS camera fallback depth"),
                size: wgpu::Extent3d {
                    width: 1,
                    height: 1,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::R16Float,
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                view_formats: &[],
            },
            wgpu::util::TextureDataOrder::LayerMajor,
            &[0, 0],
        );
        Self {
            pending,
            current: None,
            targets: ColorDepthTargets::new(device, "iOSFrameNode"),
            output_size: [1, 1],
            pipeline,
            layout,
            sampler: create_sampler_linear(device).sampler,
            fallback_depth,
        }
    }
    fn receive(&mut self, context: &RenderContext) -> bool {
        let Some(frame) = self.pending.lock().unwrap().take() else {
            return false;
        };
        self.output_size = frame.output_size();
        match unsafe { ImportedFrame::new(context, frame) } {
            Ok(imported) => self.current = Some(imported),
            Err(e) => log::error!("Metal camera import failed: {e}"),
        }
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
        original: &mut Option<Texture>,
    ) -> NodeSlots {
        // Slot negotiation happens before `render` consumes the first pending frame.
        // Read its dimensions here so the camera conversion target is not left at
        // the 1x1 placeholder size.
        if let Some(size) = self
            .pending
            .lock()
            .unwrap()
            .as_ref()
            .map(CameraFrame::output_size)
        {
            self.output_size = size;
        }
        let slots = slots.emplace_color_depth_output(
            context,
            self.output_size[0],
            self.output_size[1],
            "iOSFrameNode",
        );
        self.targets = slots.as_color_depth_targets();
        let (color, _) = slots.as_color_depth_target();
        original.replace(color.as_texture());
        slots
    }
    fn input(&mut self, eye: &EyeInput, _: &MouseInput) -> (EyeInput, NodeChanges) {
        let changed = self.receive_dummy();
        (
            eye.clone(),
            if changed {
                NodeChanges::OUTPUT
            } else {
                NodeChanges::empty()
            },
        )
    }
    fn render(
        &mut self,
        context: &RenderContext,
        encoder: &mut wgpu::CommandEncoder,
        screen: Option<&RenderTexture>,
    ) {
        self.receive(context);
        let Some(frame) = &self.current else { return };
        let target = screen.unwrap_or(&self.targets.rt_color);
        let bind = frame.bind_group(
            context.device(),
            &self.layout,
            &self.sampler,
            &self.fallback_depth,
        );
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("iOS camera conversion"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &target.view,
                resolve_target: None,
                depth_slice: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: self.targets.depth_attachment(),
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &bind, &[]);
        pass.draw(0..3, 0..1);
    }
}
impl FrameNode {
    fn receive_dummy(&self) -> bool {
        self.pending.lock().unwrap().is_some()
    }
}

struct ImportedFrame {
    _owner: CameraFrame,
    luma: wgpu::Texture,
    chroma: wgpu::Texture,
    depth: Option<wgpu::Texture>,
    uniform: wgpu::Buffer,
}
impl ImportedFrame {
    unsafe fn new(context: &RenderContext, owner: CameraFrame) -> Result<Self, String> {
        let device = context.device();
        let hal = device
            .as_hal::<wgpu_hal::api::Metal>()
            .ok_or("wgpu is not using Metal")?;
        let luma = wrap(
            device,
            owner.luma,
            wgpu::TextureFormat::R8Unorm,
            owner.width,
            owner.height,
        )?;
        let chroma = wrap(
            device,
            owner.chroma,
            wgpu::TextureFormat::Rg8Unorm,
            (owner.width + 1) / 2,
            (owner.height + 1) / 2,
        )?;
        let depth = owner
            .depth
            .map(|depth| {
                wrap(
                    device,
                    depth,
                    wgpu::TextureFormat::R16Float,
                    owner.depth_width,
                    owner.depth_height,
                )
            })
            .transpose()?;
        drop(hal);
        let values = [
            owner.rotation as f32,
            if owner.full_range { 1.0 } else { 0.0 },
            if depth.is_some() { 1.0 } else { 0.0 },
            0.0,
        ];
        use wgpu::util::DeviceExt;
        let bytes = std::slice::from_raw_parts(values.as_ptr().cast::<u8>(), 16);
        let uniform = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("camera metadata"),
            contents: bytes,
            usage: wgpu::BufferUsages::UNIFORM,
        });
        Ok(Self {
            _owner: owner,
            luma,
            chroma,
            depth,
            uniform,
        })
    }
    fn bind_group(
        &self,
        device: &wgpu::Device,
        layout: &wgpu::BindGroupLayout,
        sampler: &wgpu::Sampler,
        fallback_depth: &wgpu::Texture,
    ) -> wgpu::BindGroup {
        let y = self.luma.create_view(&Default::default());
        let uv = self.chroma.create_view(&Default::default());
        let depth = self
            .depth
            .as_ref()
            .unwrap_or(fallback_depth)
            .create_view(&Default::default());
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("camera planes"),
            layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&y),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&uv),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(&depth),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::Sampler(sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: self.uniform.as_entire_binding(),
                },
            ],
        })
    }
}

unsafe fn wrap(
    device: &wgpu::Device,
    ptr: *mut c_void,
    format: wgpu::TextureFormat,
    width: u32,
    height: u32,
) -> Result<wgpu::Texture, String> {
    let retained = Retained::<ProtocolObject<dyn MTLTexture>>::retain(ptr.cast())
        .ok_or("cannot retain MTLTexture")?;
    let hal = wgpu_hal::metal::Device::texture_from_raw(
        retained,
        format,
        MTLTextureType::Type2D,
        1,
        1,
        wgpu_hal::CopyExtent {
            width,
            height,
            depth: 1,
        },
    );
    let desc = wgpu::TextureDescriptor {
        label: Some("CVPixelBuffer plane"),
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    };
    Ok(device.create_texture_from_hal::<wgpu_hal::api::Metal>(hal, &desc))
}
fn plane_entry(binding: u32) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Texture {
            sample_type: wgpu::TextureSampleType::Float { filterable: true },
            view_dimension: wgpu::TextureViewDimension::D2,
            multisampled: false,
        },
        count: None,
    }
}
fn normalize(v: i32) -> i32 {
    match v.rem_euclid(360) {
        45..=134 => 90,
        135..=224 => 180,
        225..=314 => 270,
        _ => 0,
    }
}
