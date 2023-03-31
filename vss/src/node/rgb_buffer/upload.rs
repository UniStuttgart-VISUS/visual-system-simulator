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

pub enum RenderResolution{
    Screen{
        output_scale: OutputScale,
        input_scale: f32,
    },
    Buffer{
        input_scale: f32,
    },
    Custom{
        res: [u32; 2],
    }
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
    texture: Option<Texture>,
    render_resolution: RenderResolution,

    pipeline: wgpu::RenderPipeline,
    uniforms: ShaderUniforms<Uniforms>,
    source_bind_group: wgpu::BindGroup,
    targets: ColorDepthTargets,
}

impl UploadRgbBuffer {
    pub fn new(surface: &Surface) -> Self {
        let device = surface.device().borrow_mut();
        let queue = surface.queue().borrow_mut();

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
            render_resolution: RenderResolution::Buffer{input_scale: 1.0},

            pipeline,
            uniforms,
            source_bind_group,
            targets: ColorDepthTargets::new(&device, "UploadNode"),
        }
    }

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
        assert_eq!(buffer.pixels_rgb.len(), (buffer.width * buffer.height * 4) as usize, "Unexpected RGBA pixel buffer size");

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

    pub fn set_render_resolution(&mut self, render_resolution: RenderResolution) {
        self.render_resolution = render_resolution;
    }

    pub fn set_flags(&mut self, flags: RgbInputFlags) {
        self.uniforms.data.flags = flags.bits();
    }
}

impl Node for UploadRgbBuffer {
   

    fn negociate_slots(&mut self, surface: &Surface, slots: NodeSlots, original_image: &mut Option<Texture>) -> NodeSlots {
        if self.buffer_upload {
            let device = surface.device().borrow_mut();
            let queue = surface.queue().borrow_mut();
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

        let (width, height) = if let Some(texture) = &self.texture {
            let tex_w = texture.width as f32;
            let mut tex_h = texture.height as f32;
            let flags = RgbInputFlags::from_bits(self.uniforms.data.flags).unwrap();
            if flags.contains(RgbInputFlags::RGBD_HORIZONTAL) {
                tex_h /= 2.0;
            }
            match self.render_resolution {
                RenderResolution::Screen {output_scale, input_scale} => {
                    let mut screen_w = surface.width() as f32;
                    let mut screen_h = surface.height() as f32;
                    let tex_aspect_ratio = tex_w / tex_h;
                    let screen_aspect_ratio = screen_w / screen_h;
                    match output_scale {
                        OutputScale::Fit => {
                            if tex_aspect_ratio > screen_aspect_ratio{ // scale down the larger side
                                screen_h *= screen_aspect_ratio / tex_aspect_ratio;
                            }else{
                                screen_w *= tex_aspect_ratio / screen_aspect_ratio;
                            }
                        },
                        OutputScale::Fill => {
                            if tex_aspect_ratio > screen_aspect_ratio{ // scale up the smaller side
                                screen_w *= tex_aspect_ratio / screen_aspect_ratio;
                            }else{
                                screen_h *= screen_aspect_ratio / tex_aspect_ratio;
                            }
                        },
                        OutputScale::Stretch => {}, // no adjustment needed
                    }
                    ((screen_w * input_scale) as u32, (screen_h * input_scale) as u32)
                },
                RenderResolution::Buffer {input_scale} => {
                    ((tex_w * input_scale) as u32, (tex_h * input_scale) as u32)
                },
                RenderResolution::Custom { res } => {
                    (res[0], res[1])
                },
            }
        }else{
            (1, 1)
        };

        let slots = slots.emplace_color_depth_output(surface, width, height, "UploadNode");
        self.targets = slots.as_all_target();

        let (color_out, _) = slots.as_color_depth_target();
        original_image.replace(color_out.as_texture());

        slots
    }

    fn input(&mut self, perspective: &EyePerspective, _vis_param: &VisualizationParameters) -> EyePerspective {
        use cgmath::Matrix4;
        self.uniforms.data.inv_proj_view = (perspective.proj * (Matrix4::from_translation(-perspective.position) * perspective.view)).invert().unwrap().into();
        perspective.clone()
    }

    fn render(&mut self, surface: &Surface, encoder: &mut CommandEncoder, screen: Option<&RenderTexture>) {
        let queue = surface.queue().borrow_mut();
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
                    None,
                    &*self.buffer_next.pixels_rgb,
                    0,
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
