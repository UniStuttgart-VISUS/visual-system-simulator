use gfx;
use gfx_device_gl::Resources;

use std::io::Cursor;
use std::path::Path;

use crate::pipeline::*;

gfx_defines! {
    pipeline pipe {
        u_flags: gfx::Global<u32> = "u_flags",
        u_head: gfx::Global<[[f32; 4];4]> = "u_head",
        u_fov: gfx::Global<[f32; 2]> = "u_fov",
        s_rgb: gfx::TextureSampler<[f32; 4]> = "s_rgb",
        rt_color: gfx::RenderTarget<ColorFormat> = "rt_color",
        rt_depth: gfx::RenderTarget<DepthFormat> = "rt_depth",
    }
}

bitflags! {
    pub struct RgbInputFlags : u32 {
        const EQUIRECTANGULAR = 1;
        const VERTICALLY_FLIPPED = 2;
        const RGBD_HORIZONTAL = 4;
    }
}

/// A device for static RGBA image data.
pub struct UploadRgbBuffer {
    buffer_next: RgbBuffer,
    buffer_upload: bool,
    texture: Option<gfx::handle::Texture<Resources, RgbSurfaceFormat>>,

    pso: gfx::PipelineState<Resources, pipe::Meta>,
    pso_data: pipe::Data<Resources>,
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
        let img = reader.decode().unwrap().flipv().to_rgba();
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
            let info = texture.get_info().to_image_info(0);
            if buffer.width != info.width as u32 || buffer.height != info.height as u32 {
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

    pub fn set_flags(&mut self, flags: RgbInputFlags) {
        self.pso_data.u_flags = flags.bits();
    }
}

impl Node for UploadRgbBuffer {
    fn new(window: &Window) -> Self {
        let mut factory = window.factory().borrow_mut();

        let pso = factory
            .create_pipeline_simple(
                &include_glsl!("../shader.vert"),
                &include_glsl!("shader.frag"),
                pipe::new(),
            )
            .unwrap();

        let sampler = factory.create_sampler_linear();
        let (_, rgb_view) = load_texture_from_bytes(&mut factory, &[0; 4], 1, 1).unwrap();
        let (_, _, rt_color) = factory.create_render_target(1, 1).unwrap();
        let (_, _, rt_depth) = factory.create_render_target(1, 1).unwrap();

        UploadRgbBuffer {
            buffer_next: RgbBuffer::default(),
            buffer_upload: false,
            texture: None,

            pso,
            pso_data: pipe::Data {
                u_flags: RgbInputFlags::empty().bits(),
                u_head: [[0.0; 4]; 4],
                u_fov: [90.0_f32.to_radians(), 59.0_f32.to_radians()],
                s_rgb: (rgb_view, sampler),
                rt_color,
                rt_depth,
            },
        }
    }

    fn update_io(
        &mut self,
        window: &Window,
        _source: (Option<DeviceSource>, Option<DeviceTarget>),
        _target_candidate: (Option<DeviceSource>, Option<DeviceTarget>),
    ) -> (Option<DeviceSource>, Option<DeviceTarget>) {
        if self.buffer_upload {
            let mut factory = window.factory().borrow_mut();
            let (texture, view) = load_texture_from_bytes(
                &mut factory,
                &self.buffer_next.pixels_rgb,
                self.buffer_next.width as u32,
                self.buffer_next.height as u32,
            )
            .unwrap();
            self.texture = Some(texture);

            let sampler = factory.create_sampler_linear();
            self.pso_data.s_rgb = (view, sampler.clone());
        }

        let mut width = 1;
        let mut height = 1;
        if let Some(texture) = &self.texture {
            let info = texture.get_info().to_image_info(0);
            width = info.width as u32;
            height = info.height as u32;
        }

        let flags = RgbInputFlags::from_bits(self.pso_data.u_flags).unwrap();
        if flags.contains(RgbInputFlags::RGBD_HORIZONTAL) {
            height /= 2;
        }

        // Compute vertical FOV from aspect ratio.
        self.pso_data.u_fov[1] =
            2.0 * ((self.pso_data.u_fov[0] / 2.0).tan() * height as f32 / width as f32).atan();

        let (color_view, color) = create_texture_render_target::<ColorFormat>(
            &mut window.factory().borrow_mut(),
            width,
            height,
        );
        let (depth_view, depth) = create_texture_render_target::<DepthFormat>(
            &mut window.factory().borrow_mut(),
            width,
            height,
        );

        self.pso_data.rt_color = color.clone();
        self.pso_data.rt_depth = depth.clone();
        (
            Some(DeviceSource::RgbDepth {
                width,
                height,
                rgba8: color_view,
                d: depth_view,
            }),
            Some(color),
        )
    }

    fn input(&mut self, head: &Head, gaze: &DeviceGaze) -> DeviceGaze {
        use cgmath::Matrix4;
        self.pso_data.u_head = (Matrix4::from_angle_y(cgmath::Rad(head.yaw))
            * Matrix4::from_angle_x(cgmath::Rad(head.pitch)))
        .into();
        gaze.clone()
    }

    fn render(&mut self, window: &Window) {
        let mut encoder = window.encoder().borrow_mut();

        if let Some(texture) = &self.texture {
            if self.buffer_upload {
                update_texture(
                    &mut encoder,
                    &texture,
                    [
                        self.buffer_next.width as u16,
                        self.buffer_next.height as u16,
                    ],
                    [0, 0],
                    &*self.buffer_next.pixels_rgb,
                );
                self.buffer_upload = false;
            }
        }

        encoder.draw(&gfx::Slice::from_vertex_count(6), &self.pso, &self.pso_data);
    }
}
