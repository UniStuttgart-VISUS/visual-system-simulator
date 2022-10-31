use super::*;
use std::io::Cursor;
use std::path::Path;
use wgpu::CommandEncoder;

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
    uniforms: ShaderUniforms<Uniforms>,
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
            if buffer.width != texture.width || buffer.height != texture.height {
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
        // self.uniforms.flags = flags.bits();
    }
}

impl Node for UploadRgbBuffer {
    fn new(window: &Window) -> Self {
        let device = window.device().borrow_mut();
        let queue = window.queue().borrow_mut();

        let uniforms = ShaderUniforms::new(&device, 
            Uniforms{
                flags: RgbInputFlags::empty().bits(),
                proj_view: [[0.0; 4]; 4],
            });
        
        let s_rgb = placeholder_texture(&device, &queue, Some("UploadNode s_rgb")).unwrap();
        let (s_rgb_bind_group_layout, s_rgb_bind_group) = s_rgb.create_bind_group(&device);

        let rt_color = placeholder_color_rt(&device, Some("UploadNode rt_color"));
        let rt_depth = placeholder_depth_rt(&device, Some("UploadNode rt_depth"));
        let rt_deflection = placeholder_highp_rt(&device, Some("UploadNode rt_deflection"));
        let rt_color_change = placeholder_highp_rt(&device, Some("UploadNode rt_color_change"));
        let rt_color_uncertainty = placeholder_highp_rt(&device, Some("UploadNode rt_color_uncertainty"));
        let rt_covariances = placeholder_highp_rt(&device, Some("UploadNode rt_covariances"));

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("UploadNode Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("upload.wgsl").into()),
        });
        
        let pipeline = create_render_pipeline(
            &device,
            &[&shader, &shader],
            &["vs_main", "fs_main"],
            &[&s_rgb_bind_group_layout, &uniforms.bind_group_layout],
            &[
                blended_color_state(ColorFormat), 
                simple_color_state(HighpFormat),
                simple_color_state(HighpFormat),
                simple_color_state(HighpFormat),
                simple_color_state(HighpFormat),
                ],
            simple_depth_state(DepthFormat),
            Some("UploadNode Render Pipeline"));

        UploadRgbBuffer {
            buffer_next: RgbBuffer::default(),
            buffer_upload: false,
            texture: None,
            render_resolution: None,

            pipeline,
            uniforms,
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
        }
    }

    fn negociate_slots(&mut self, window: &Window, slots: NodeSlots) -> NodeSlots {
        if self.buffer_upload {
            let device = window.device().borrow_mut();
            let queue = window.queue().borrow_mut();
            let sampler = create_sampler_linear(&device);
            let texture = load_texture_from_bytes(
                &device,
                &queue,
                &self.buffer_next.pixels_rgb,
                self.buffer_next.width as u32,
                self.buffer_next.height as u32,
                sampler,
                wgpu::TextureFormat::Rgba8Unorm,
                Some("UploadNode s_rgb"),
            )
            .unwrap();
            (_, self.sources.s_rgb_bind_group) = texture.create_bind_group(&device);
            self.texture = Some(texture);
        }

        let mut width = 1;
        let mut height = 1;
        if let Some(resolution) = &self.render_resolution {
            width = resolution[0];
            height = resolution[1];
        }else{
            if let Some(texture) = &self.texture {
                width = texture.width;
                height = texture.height;
            }
    
            let flags = RgbInputFlags::from_bits(self.uniforms.data.flags).unwrap();
            if flags.contains(RgbInputFlags::RGBD_HORIZONTAL) {
                height /= 2;
            }
        }

        let slots = slots.emplace_color_depth_output(window, width, height);
        (self.targets.rt_color,
         self.targets.rt_depth,
         self.targets.rt_deflection,
         self.targets.rt_color_change,
         self.targets.rt_color_uncertainty,
         self.targets.rt_covariances) = slots.as_all_output();

        slots
    }

    fn input(&mut self, perspective: &EyePerspective, _vis_param: &VisualizationParameters) -> EyePerspective {
        use cgmath::Matrix4;
        self.uniforms.data.proj_view = (perspective.proj * (Matrix4::from_translation(-perspective.position) * perspective.view)).into();
        perspective.clone()
    }

    fn render(&mut self, window: &window::Window, encoder: &mut CommandEncoder, screen: &RenderTexture) {
        let queue = window.queue().borrow_mut();
        self.uniforms.update(&queue);

        if let Some(texture) = &self.texture {
            if self.buffer_upload {
                update_texture(
                    &queue,
                    &texture,
                    [
                        self.buffer_next.width,
                        self.buffer_next.height,
                    ],
                    // [0, 0],
                    &*self.buffer_next.pixels_rgb,
                );
                self.buffer_upload = false;
            }
        }

        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("uploadnode_render_pass"),
            color_attachments: &[
                self.targets.rt_color.to_color_attachment(),
                self.targets.rt_deflection.to_color_attachment(),
                self.targets.rt_color_change.to_color_attachment(),
                self.targets.rt_color_uncertainty.to_color_attachment(),
                self.targets.rt_covariances.to_color_attachment(),
                ],
            depth_stencil_attachment: self.targets.rt_depth.to_depth_attachment(),
        });
    
        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, &self.sources.s_rgb_bind_group, &[]);
        render_pass.set_bind_group(1, &self.uniforms.bind_group, &[]);
        render_pass.draw(0..6, 0..1);
    }
}
