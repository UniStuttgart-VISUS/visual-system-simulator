use super::*;
// use gfx;
use std::io::Cursor;
use std::path::Path;
use wgpu::{util::DeviceExt, CommandEncoder};
// use gfx::format::Rgba32F;

// gfx_defines! {
//     pipeline pipe {
//         u_flags: gfx::Global<u32> = "u_flags",
//         u_proj_view: gfx::Global<[[f32; 4];4]> = "u_proj_view",
//         s_rgb: gfx::TextureSampler<[f32; 4]> = "s_rgb",
//         rt_color: gfx::RenderTarget<ColorFormat> = "rt_color",
//         rt_depth: gfx::RenderTarget<DepthFormat> = "rt_depth",
//         rt_deflection: gfx::RenderTarget<Rgba32F> = "rt_deflection",
//         rt_color_change: gfx::RenderTarget<Rgba32F> = "rt_color_change",
//         rt_color_uncertainty: gfx::RenderTarget<Rgba32F> = "rt_color_uncertainty",
//         rt_covariances: gfx::RenderTarget<Rgba32F> = "rt_covariances",
//     }
// }

struct Uniforms{
    flags: u32,
    proj_view: [[f32; 4];4],
}

struct Sources{
    s_rgb: texture::Texture,
    s_rgb_bind_group: wgpu::BindGroup,
}

struct Targets{
    rt_color: RenderTexture,
    rt_depth: RenderTexture,
    rt_deflection: RenderTexture,
    rt_color_change: RenderTexture,
    rt_color_uncertainty: RenderTexture,
    rt_covariances: RenderTexture,
    // rt_color: gfx::RenderTarget<ColorFormat> = "rt_color",
    // rt_depth: gfx::RenderTarget<DepthFormat> = "rt_depth",
    // rt_deflection: gfx::RenderTarget<Rgba32F> = "rt_deflection",
    // rt_color_change: gfx::RenderTarget<Rgba32F> = "rt_color_change",
    // rt_color_uncertainty: gfx::RenderTarget<Rgba32F> = "rt_color_uncertainty",
    // rt_covariances: gfx::RenderTarget<Rgba32F> = "rt_covariances",
}

bitflags! {
    pub struct RgbInputFlags : u32 {
        const EQUIRECTANGULAR = 1;
        const VERTICALLY_FLIPPED = 2;
        const RGBD_HORIZONTAL = 4;
    }
}

impl RgbInputFlags {
    pub fn from_extension<P>(path: P) -> RgbInputFlags
    where
        P: AsRef<Path>,
    {
        let mut flags = RgbInputFlags::empty();
        let file_name = path
            .as_ref()
            .file_name()
            .unwrap()
            .to_os_string()
            .into_string()
            .unwrap();
        if file_name.contains(".rgbd.") {
            flags |= RgbInputFlags::RGBD_HORIZONTAL;
        }
        if file_name.contains(".erp.") {
            flags |= RgbInputFlags::EQUIRECTANGULAR;
        }
        flags
    }
}

/// A device for static RGBA image data.
pub struct UploadRgbBuffer {
    buffer_next: RgbBuffer,
    buffer_upload: bool,
    texture: Option<Texture>,//Option<gfx::handle::Texture<Resources, gfx::format::R8_G8_B8_A8>>,
    render_resolution: Option<[u32; 2]>,

    pipeline: wgpu::RenderPipeline,
    uniforms: Uniforms,
    uniforms_buffer: wgpu::Buffer,
    uniforms_bind_group: wgpu::BindGroup,
    sources: Sources,
    targets: Targets,
    // pso: gfx::PipelineState<Resources, pipe::Meta>,
    // pso_data: pipe::Data<Resources>,
}

impl UploadRgbBuffer {
    pub fn has_image_extension<P>(path: P) -> bool
    where
        P: AsRef<Path>,
    {
        image::ImageFormat::from_path(path).is_ok()
    }

    pub fn upload_image(&mut self, cursor: Cursor<Vec<u8>>) {
        let reader = image::io::Reader::new(cursor)
            .with_guessed_format()
            .expect("Cursor io never fails");
        let img = reader.decode().unwrap().flipv().to_rgba8();
        let (width, height) = img.dimensions();

        self.upload_buffer(&RgbBuffer {
            pixels_rgb: img.into_raw().into_boxed_slice(),
            width,
            height,
        });
    }

    pub fn upload_buffer(&mut self, buffer: &RgbBuffer) {
        // Test if we have to invalidate the texture.
        if let Some(texture) = &self.texture {
            if buffer.width != texture.width as u32 || buffer.height != texture.height as u32 {
                self.texture = None;
            }
        }

        if self.buffer_next.width != buffer.width || self.buffer_next.height != buffer.height {
            // Reallocate and copy.
            self.buffer_next = RgbBuffer {
                pixels_rgb: buffer.pixels_rgb.clone(),
                width: buffer.width,
                height: buffer.height,
            }
        } else {
            // Copy.
            self.buffer_next
                .pixels_rgb
                .copy_from_slice(&buffer.pixels_rgb);
        }

        self.buffer_upload = true;
    }

    pub fn set_render_resolution(&mut self, render_resolution: Option<[u32; 2]>) {
        self.render_resolution = render_resolution;
    }

    pub fn set_flags(&mut self, flags: RgbInputFlags) {
        self.uniforms.flags = flags.bits();
    }
}

impl Node for UploadRgbBuffer {
    fn new(window: &Window) -> Self {
        let device = window.device().borrow_mut();
        let queue = window.queue().borrow_mut();

        // let pso = factory
        //     .create_pipeline_simple(
        //         &include_glsl!("../mod.vert"),
        //         &include_glsl!("upload.frag"),
        //         pipe::new(),
        //     )
        //     .unwrap();

        let uniforms = Uniforms{
            flags: RgbInputFlags::empty().bits(),
            proj_view: [[0.0; 4]; 4],
        };

        let uniforms_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Uniform Buffer"),
            contents: unsafe { any_as_u8_slice(&uniforms) },
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let uniforms_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
                label: Some("uniforms_bind_group_layout"),
            });

        let uniforms_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &uniforms_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniforms_buffer.as_entire_binding(),
            }],
            label: Some("uniforms_bind_group"),
        });

        let s_rgb = load_texture_from_bytes(&device, &queue, &[0; 4], 1, 1, Some("UploadNode s_rgb")).unwrap();
        let sampler = create_sampler_linear(&device).unwrap();
        let (s_rgb_bind_group_layout, s_rgb_bind_group) = s_rgb.create_bind_group(&device, &sampler);

        let rt_color = create_texture_render_target(&device, &queue, 1, 1, ColorFormat, Some("UploadNode rt_color"));
        let rt_depth = create_texture_render_target(&device, &queue, 1, 1, DepthFormat, Some("UploadNode rt_depth"));
        let rt_deflection = create_texture_render_target(&device, &queue, 1, 1, HighpFormat, Some("UploadNode rt_deflection"));
        let rt_color_change = create_texture_render_target(&device, &queue, 1, 1, HighpFormat, Some("UploadNode rt_color_change"));
        let rt_color_uncertainty = create_texture_render_target(&device, &queue, 1, 1, HighpFormat, Some("UploadNode rt_color_uncertainty"));
        let rt_covariances = create_texture_render_target(&device, &queue, 1, 1, HighpFormat, Some("UploadNode rt_covariances"));

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("TestNode Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("upload.wgsl").into()),
        });

        let pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("TestNode Render Pipeline Layout"),
                bind_group_layouts: &[&s_rgb_bind_group_layout, &uniforms_bind_group_layout],
                push_constant_ranges: &[],
        });
        
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("TestNode Render Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: window.surface_config().borrow().format,
                    blend: Some(wgpu::BlendState {
                        color: wgpu::BlendComponent::REPLACE,
                        alpha: wgpu::BlendComponent::REPLACE,
                    }),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                // Setting this to anything other than Fill requires Features::POLYGON_MODE_LINE
                // or Features::POLYGON_MODE_POINT
                polygon_mode: wgpu::PolygonMode::Fill,
                // Requires Features::DEPTH_CLIP_CONTROL
                unclipped_depth: false,
                // Requires Features::CONSERVATIVE_RASTERIZATION
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            // If the pipeline will be used with a multiview render pass, this
            // indicates how many array layers the attachments will have.
            multiview: None,
        });

        UploadRgbBuffer {
            buffer_next: RgbBuffer::default(),
            buffer_upload: false,
            texture: None,
            render_resolution: None,

            pipeline,
            uniforms,
            uniforms_buffer,
            uniforms_bind_group,
            sources: Sources{
                s_rgb,
                s_rgb_bind_group,
            },
            targets: Targets{
                rt_color,
                rt_depth,
                rt_deflection,
                rt_color_change,
                rt_color_uncertainty,
                rt_covariances,
            },
            // pso,
            // pso_data: pipe::Data {
            //     u_flags: RgbInputFlags::empty().bits(),
            //     u_proj_view: [[0.0; 4]; 4],
            //     s_rgb: (rgb_view, sampler),
            //     rt_color,
            //     rt_depth,
            //     rt_deflection,
            //     rt_color_change,
            //     rt_color_uncertainty,
            //     rt_covariances
            // },
        }
    }

    // fn negociate_slots(&mut self, window: &Window, slots: NodeSlots) -> NodeSlots {
    //     if self.buffer_upload {
    //         let mut factory = window.factory().borrow_mut();
    //         let (texture, view) = load_texture_from_bytes(
    //             &mut factory,
    //             &self.buffer_next.pixels_rgb,
    //             self.buffer_next.width as u32,
    //             self.buffer_next.height as u32,
    //         )
    //         .unwrap();
    //         self.texture = Some(texture);

    //         let sampler = factory.create_sampler_linear();
    //         self.pso_data.s_rgb = (view, sampler.clone());
    //     }

    //     let mut width = 1;
    //     let mut height = 1;
    //     if let Some(resolution) = &self.render_resolution {
    //         width = resolution[0];
    //         height = resolution[1];
    //     }else{
    //         if let Some(texture) = &self.texture {
    //             let info = texture.get_info().to_image_info(0);
    //             width = info.width as u32;
    //             height = info.height as u32;
    //         }
    
    //         let flags = RgbInputFlags::from_bits(self.pso_data.u_flags).unwrap();
    //         if flags.contains(RgbInputFlags::RGBD_HORIZONTAL) {
    //             height /= 2;
    //         }
    //     }

    //     // Compute vertical FOV from aspect ratio.
    //     self.pso_data.u_fov[1] =
    //         2.0 * ((self.pso_data.u_fov[0] / 2.0).tan() * height as f32 / width as f32).atan();

    //     let slots = slots.emplace_color_depth_output(window, width, height);
    //     let (color, depth, deflection, color_change, color_uncertainty, covariances) = slots.as_all_output();
    //     self.pso_data.rt_color = color;
    //     self.pso_data.rt_depth = depth;
    //     self.pso_data.rt_deflection = deflection;
    //     self.pso_data.rt_color_change = color_change;
    //     self.pso_data.rt_color_uncertainty = color_uncertainty;
    //     self.pso_data.rt_covariances = covariances;

    //     slots
    // }

    fn input(&mut self, perspective: &EyePerspective, _vis_param: &VisualizationParameters) -> EyePerspective {
        use cgmath::Matrix4;
        self.uniforms.proj_view = (perspective.proj * (Matrix4::from_translation(-perspective.position) * perspective.view)).into();
        perspective.clone()
    }

    fn render(&mut self, window: &window::Window, encoder: &mut CommandEncoder, screen: &RenderTexture) {
        let mut render_pass = create_render_pass(encoder, &self.targets.rt_color);

        // if let Some(texture) = &self.texture {
        //     if self.buffer_upload {
        //         update_texture(
        //             &mut encoder,
        //             &texture,
        //             [
        //                 self.buffer_next.width as u16,
        //                 self.buffer_next.height as u16,
        //             ],
        //             [0, 0],
        //             &*self.buffer_next.pixels_rgb,
        //         );
        //         self.buffer_upload = false;
        //     }
        // }

        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, &self.sources.s_rgb_bind_group, &[]);
        render_pass.set_bind_group(1, &self.uniforms_bind_group, &[]);
        render_pass.draw(0..6, 0..1);
    }
}
