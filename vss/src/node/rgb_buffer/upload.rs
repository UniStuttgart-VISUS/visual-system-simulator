use super::*;
use std::io::Cursor;
use std::path::Path;
use cgmath::SquareMatrix;
use wgpu::CommandEncoder;

struct Uniforms{
    inv_proj_view: [[f32; 4];4],
    flags: u32,
    _padding: [i32; 3],
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
    source_bind_group: wgpu::BindGroup,
    targets: ColorDepthTargets,
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
        self.uniforms.data.flags = flags.bits();
    }
}

impl Node for UploadRgbBuffer {
    fn new(window: &Window) -> Self {
        let device = window.device().borrow_mut();
        let queue = window.queue().borrow_mut();

        let uniforms = ShaderUniforms::new(&device, 
            Uniforms{
                inv_proj_view: [[0.0; 4]; 4],
                flags: RgbInputFlags::empty().bits(),
                _padding: [0; 3],
            });
        
        let source_texture = placeholder_texture(&device, &queue, Some("UploadNode source_texture")).unwrap();
        let (source_bind_group_layout, source_bind_group) = source_texture.create_bind_group(&device);

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("UploadNode Shader"),
            source: wgpu::ShaderSource::Wgsl(concat!(
                include_str!("../vert.wgsl"),
                include_str!("upload.wgsl")).into()),
        });
        
        let pipeline = create_render_pipeline(
            &device,
            &[&shader, &shader],
            &["vs_main", "fs_main"],
            &[&source_bind_group_layout, &uniforms.bind_group_layout],
            &all_color_states(),
            simple_depth_state(DEPTH_FORMAT),
            Some("UploadNode Render Pipeline"));

        UploadRgbBuffer {
            buffer_next: RgbBuffer::default(),
            buffer_upload: false,
            texture: None,
            render_resolution: None,

            pipeline,
            uniforms,
            source_bind_group,
            targets: ColorDepthTargets::new(&device, "UploadNode"),
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
            (_, self.source_bind_group) = texture.create_bind_group(&device);
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

        let slots = slots.emplace_color_depth_output(window, width, height, "UploadNode");
        self.targets = slots.as_all_target();

        slots
    }

    fn input(&mut self, perspective: &EyePerspective, _vis_param: &VisualizationParameters) -> EyePerspective {
        use cgmath::Matrix4;
        self.uniforms.data.inv_proj_view = (perspective.proj * (Matrix4::from_translation(-perspective.position) * perspective.view)).invert().unwrap().into();
        perspective.clone()
    }

    fn render(&mut self, window: &window::Window, encoder: &mut CommandEncoder, screen: Option<&RenderTexture>) {
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
            label: Some("UploadNode render_pass"),
            color_attachments: &self.targets.color_attachments(screen),
            depth_stencil_attachment: self.targets.depth_attachment(),
        });
    
        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, &self.source_bind_group, &[]);
        render_pass.set_bind_group(1, &self.uniforms.bind_group, &[]);
        render_pass.draw(0..6, 0..1);
    }
}
