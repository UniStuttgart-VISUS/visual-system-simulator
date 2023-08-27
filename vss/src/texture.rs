use wgpu::Origin3d;

use crate::*;
use std::io::Cursor;
use std::rc::Rc;

pub struct BufferDimensions {
    pub width: usize,
    pub height: usize,
    pub unpadded_bytes_per_row: usize,
    pub padded_bytes_per_row: usize,
}

pub struct Texture {
    //TODO maybe use RefCell
    pub texture: Rc<wgpu::Texture>,
    pub view: Rc<wgpu::TextureView>,
    pub sampler: Rc<Sampler>,
    pub view_dimension: wgpu::TextureViewDimension,
    pub label: String,
}

pub struct RenderTexture {
    pub texture: Option<Rc<wgpu::Texture>>,
    pub view: Rc<wgpu::TextureView>,
    pub sampler: Rc<Sampler>,
    pub view_dimension: wgpu::TextureViewDimension,
    pub width: u32,
    pub height: u32,
    pub label: String,
}

pub struct Sampler {
    pub sampler: wgpu::Sampler,
    pub binding_type: wgpu::SamplerBindingType,
    pub filterable: bool,
}

impl Texture {
    pub fn create_bind_group(
        &self,
        device: &wgpu::Device,
    ) -> (wgpu::BindGroupLayout, wgpu::BindGroup) {
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: self.view_dimension,
                        sample_type: wgpu::TextureSampleType::Float {
                            filterable: self.sampler.filterable,
                        },
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(self.sampler.binding_type),
                    count: None,
                },
            ],
            label: Some("texture_bind_group_layout"),
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(self.view.as_ref()),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.sampler.sampler),
                },
            ],
            label: Some("texture_bind_group"),
        });

        (layout, bind_group)
    }

    pub fn width(&self) -> u32 {
        self.texture.width()
    }

    pub fn height(&self) -> u32 {
        self.texture.height()
    }
}

impl Clone for Texture {
    fn clone(&self) -> Self {
        Texture {
            texture: self.texture.clone(),
            view: self.view.clone(),
            sampler: self.sampler.clone(),
            label: format!("{}{}", self.label, " (Clone)"),
            ..*self
        }
    }
}

pub trait TextureSliceMethods {
    fn create_bind_group(&self, device: &wgpu::Device) -> (wgpu::BindGroupLayout, wgpu::BindGroup);
}

impl TextureSliceMethods for [&Texture] {
    fn create_bind_group(&self, device: &wgpu::Device) -> (wgpu::BindGroupLayout, wgpu::BindGroup) {
        //TODO switch sampler and texture order to keep the bind group order
        // the same when using a single texture or multiple

        let mut layout_entries: Vec<wgpu::BindGroupLayoutEntry> = Vec::new();

        layout_entries.extend(
            self.iter()
                .enumerate()
                .map(|(i, t)| wgpu::BindGroupLayoutEntry {
                    binding: (i * 2) as u32,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(t.sampler.binding_type),
                    count: None,
                }),
        );

        layout_entries.extend(
            self.iter()
                .enumerate()
                .map(|(i, t)| wgpu::BindGroupLayoutEntry {
                    binding: (i * 2 + 1) as u32,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        sample_type: wgpu::TextureSampleType::Float {
                            filterable: t.sampler.filterable,
                        },
                    },
                    count: None,
                }),
        );

        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &layout_entries,
            label: Some("textures_bind_group_layout"),
        });

        let mut group_entries: Vec<wgpu::BindGroupEntry> = Vec::new();

        group_entries.extend(self.iter().enumerate().map(|(i, t)| wgpu::BindGroupEntry {
            binding: (i * 2) as u32,
            resource: wgpu::BindingResource::Sampler(&t.sampler.sampler),
        }));

        group_entries.extend(self.iter().enumerate().map(|(i, t)| wgpu::BindGroupEntry {
            binding: (i * 2 + 1) as u32,
            resource: wgpu::BindingResource::TextureView(&t.view),
        }));

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &layout,
            entries: &group_entries,
            label: Some("textures_bind_group"),
        });

        (layout, bind_group)
    }
}

impl RenderTexture {
    pub fn create_depth(
        device: &wgpu::Device,
        width: u32,
        height: u32,
        label: Option<&str>,
    ) -> RenderTexture {
        Self::create(
            device,
            width,
            height,
            DEPTH_FORMAT,
            create_sampler_nearest(device),
            label,
        )
    }

    pub fn create_color(
        device: &wgpu::Device,
        width: u32,
        height: u32,
        label: Option<&str>,
    ) -> RenderTexture {
        Self::create(
            device,
            width,
            height,
            COLOR_FORMAT,
            create_sampler_linear(device),
            label,
        )
    }

    pub fn create_highp(
        device: &wgpu::Device,
        width: u32,
        height: u32,
        label: Option<&str>,
    ) -> RenderTexture {
        Self::create(
            device,
            width,
            height,
            HIGHP_FORMAT,
            create_sampler_nearest(device),
            label,
        )
    }

    pub fn empty_depth(device: &wgpu::Device, label: Option<&str>) -> RenderTexture {
        Self::create(
            device,
            1,
            1,
            DEPTH_FORMAT,
            create_sampler_nearest(device),
            label,
        )
    }

    pub fn empty_color(device: &wgpu::Device, label: Option<&str>) -> RenderTexture {
        Self::create(
            device,
            1,
            1,
            COLOR_FORMAT,
            create_sampler_linear(device),
            label,
        )
    }

    pub fn empty_highp(device: &wgpu::Device, label: Option<&str>) -> RenderTexture {
        Self::create(
            device,
            1,
            1,
            HIGHP_FORMAT,
            create_sampler_nearest(device),
            label,
        )
    }

    /// Creates a texture that can be read from in shaders (view) and rendered to (render target).
    fn create(
        device: &wgpu::Device,
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

        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label,
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[format],
        });

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

    pub fn create_bind_group(
        &self,
        device: &wgpu::Device,
    ) -> (wgpu::BindGroupLayout, wgpu::BindGroup) {
        self.as_texture().create_bind_group(device)
    }

    pub fn as_texture(&self) -> Texture {
        Texture {
            texture: self.texture.clone().unwrap(),
            view: self.view.clone(),
            sampler: self.sampler.clone(),
            view_dimension: self.view_dimension,
            label: format!("{}{}", self.label, " (Clone from RenderTexture)"),
        }
    }

    pub fn to_color_attachment(
        &self,
        clear: Option<wgpu::Color>,
    ) -> Option<wgpu::RenderPassColorAttachment> {
        Some(wgpu::RenderPassColorAttachment {
            view: self.view.as_ref(),
            resolve_target: None,
            ops: wgpu::Operations {
                load: if let Some(clear_color) = clear {
                    wgpu::LoadOp::Clear(clear_color)
                } else {
                    wgpu::LoadOp::Load
                },
                store: true,
            },
        })
    }

    pub fn to_depth_attachment(
        &self,
        clear: Option<f32>,
    ) -> Option<wgpu::RenderPassDepthStencilAttachment> {
        Some(wgpu::RenderPassDepthStencilAttachment {
            view: self.view.as_ref(),
            depth_ops: Some(wgpu::Operations {
                load: if let Some(clear_depth) = clear {
                    wgpu::LoadOp::Clear(clear_depth)
                } else {
                    wgpu::LoadOp::Load
                },
                store: true,
            }),
            stencil_ops: None,
        })
    }
}

impl Clone for RenderTexture {
    fn clone(&self) -> RenderTexture {
        RenderTexture {
            texture: self.texture.clone(),
            view: self.view.clone(),
            sampler: self.sampler.clone(),
            view_dimension: self.view_dimension,
            width: self.width,
            height: self.height,
            label: format!("{}{}", self.label, " (Clone)"),
        }
    }
}

impl BufferDimensions {
    pub fn new(width: usize, height: usize, bytes_per_pixel: usize) -> Self {
        let unpadded_bytes_per_row = width * bytes_per_pixel;
        let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT as usize;
        let padded_bytes_per_row_padding = (align - unpadded_bytes_per_row % align) % align;
        let padded_bytes_per_row = unpadded_bytes_per_row + padded_bytes_per_row_padding;
        Self {
            width,
            height,
            unpadded_bytes_per_row,
            padded_bytes_per_row,
        }
    }
}

pub fn create_color_sources_bind_group(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    node_name: &str,
) -> (wgpu::BindGroupLayout, wgpu::BindGroup) {
    [
        &placeholder_texture(
            device,
            queue,
            Some(format!("{}{}", node_name, " s_color (placeholder)").as_str()),
        )
        .unwrap(),
        &placeholder_highp_texture(
            device,
            queue,
            Some(format!("{}{}", node_name, " s_deflection (placeholder)").as_str()),
        )
        .unwrap(),
        &placeholder_highp_texture(
            device,
            queue,
            Some(format!("{}{}", node_name, " s_color_change (placeholder)").as_str()),
        )
        .unwrap(),
        &placeholder_highp_texture(
            device,
            queue,
            Some(format!("{}{}", node_name, " s_color_uncertainty (placeholder)").as_str()),
        )
        .unwrap(),
        &placeholder_highp_texture(
            device,
            queue,
            Some(format!("{}{}", node_name, " s_covariances (placeholder)").as_str()),
        )
        .unwrap(),
    ]
    .create_bind_group(device)
}

pub fn create_color_depth_sources_bind_group(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    node_name: &str,
) -> (wgpu::BindGroupLayout, wgpu::BindGroup) {
    [
        &placeholder_texture(
            device,
            queue,
            Some(format!("{}{}", node_name, " s_color (placeholder)").as_str()),
        )
        .unwrap(),
        &placeholder_depth_texture(
            device,
            Some(format!("{}{}", node_name, " s_depth (placeholder)").as_str()),
        )
        .unwrap(),
        &placeholder_highp_texture(
            device,
            queue,
            Some(format!("{}{}", node_name, " s_deflection (placeholder)").as_str()),
        )
        .unwrap(),
        &placeholder_highp_texture(
            device,
            queue,
            Some(format!("{}{}", node_name, " s_color_change (placeholder)").as_str()),
        )
        .unwrap(),
        &placeholder_highp_texture(
            device,
            queue,
            Some(format!("{}{}", node_name, " s_color_uncertainty (placeholder)").as_str()),
        )
        .unwrap(),
        &placeholder_highp_texture(
            device,
            queue,
            Some(format!("{}{}", node_name, " s_covariances (placeholder)").as_str()),
        )
        .unwrap(),
    ]
    .create_bind_group(device)
}

///
/// Can be used to replace parts of or a whole texture.
///
pub fn update_texture(
    queue: &wgpu::Queue,
    texture: &Texture,
    size: [u32; 2],
    offset: Option<Origin3d>,
    raw_data: &[u8],
    data_offset: u64,
) {
    let texture_size = wgpu::Extent3d {
        width: size[0],
        height: size[1],
        depth_or_array_layers: 1,
    };

    queue.write_texture(
        wgpu::ImageCopyTexture {
            aspect: wgpu::TextureAspect::All,
            texture: texture.texture.as_ref(),
            mip_level: 0,
            origin: offset.unwrap_or(wgpu::Origin3d::ZERO),
        },
        raw_data,
        wgpu::ImageDataLayout {
            offset: data_offset,
            bytes_per_row: Some(
                texture
                    .texture
                    .format()
                    .block_size(None)
                    .expect("combined depth-stencil format requires specifying a TextureAspect")
                    * size[0],
            ),
            rows_per_image: Some(size[1]),
        },
        texture_size,
    );
}

///
/// Load bytes as texture into GPU
///
pub fn load_texture_from_bytes(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    data: &[u8],
    width: u32,
    height: u32,
    sampler: Sampler,
    format: wgpu::TextureFormat,
    label: Option<&str>,
) -> Result<Texture, String> {
    // inspired by https://github.com/sotrh/learn-wgpu/blob/master/code/beginner/tutorial6-uniforms/src/texture.rs
    let size = wgpu::Extent3d {
        width,
        height,
        depth_or_array_layers: 1,
    };

    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label,
        size,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[format],
    });

    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

    queue.write_texture(
        wgpu::ImageCopyTexture {
            aspect: wgpu::TextureAspect::All,
            texture: &texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
        },
        data,
        wgpu::ImageDataLayout {
            offset: 0,
            bytes_per_row: Some(
                format
                    .block_size(None)
                    .expect("combined depth-stencil format requires specifying a TextureAspect")
                    * width,
            ),
            rows_per_image: Some(height),
        },
        size,
    );

    Ok(Texture {
        texture: Rc::new(texture),
        view: Rc::new(view),
        sampler: Rc::new(sampler),
        view_dimension: wgpu::TextureViewDimension::D2,
        label: label.unwrap_or("Unlabeled").to_string(),
    })
}

pub fn placeholder_texture(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    label: Option<&str>,
) -> Result<Texture, String> {
    let sampler = create_sampler_linear(device);
    load_texture_from_bytes(device, queue, &[0; 4], 1, 1, sampler, COLOR_FORMAT, label)
}

pub fn placeholder_depth_texture(
    device: &wgpu::Device,
    label: Option<&str>,
) -> Result<Texture, String> {
    let sampler = create_sampler_nearest(device);

    let size = wgpu::Extent3d {
        width: 1,
        height: 1,
        depth_or_array_layers: 1,
    };

    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label,
        size,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: DEPTH_FORMAT,
        usage: wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[DEPTH_FORMAT],
    });

    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

    Ok(Texture {
        texture: Rc::new(texture),
        view: Rc::new(view),
        sampler: Rc::new(sampler),
        view_dimension: wgpu::TextureViewDimension::D2,
        label: label.unwrap_or("Unlabeled").to_string(),
    })
}

pub fn placeholder_highp_texture(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    label: Option<&str>,
) -> Result<Texture, String> {
    let sampler = create_sampler_nearest(device);
    load_texture_from_bytes(device, queue, &[0; 16], 1, 1, sampler, HIGHP_FORMAT, label)
}

pub fn placeholder_single_channel_texture(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    label: Option<&str>,
) -> Result<Texture, String> {
    let sampler = create_sampler_linear(device);
    load_texture_from_bytes(
        device,
        queue,
        &[0; 16],
        1,
        1,
        sampler,
        wgpu::TextureFormat::R8Unorm,
        label,
    )
}

pub fn create_sampler_linear(device: &wgpu::Device) -> Sampler {
    let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        address_mode_u: wgpu::AddressMode::ClampToEdge,
        address_mode_v: wgpu::AddressMode::ClampToEdge,
        address_mode_w: wgpu::AddressMode::ClampToEdge,
        mag_filter: wgpu::FilterMode::Linear,
        min_filter: wgpu::FilterMode::Linear,
        mipmap_filter: wgpu::FilterMode::Linear,
        ..Default::default()
    });

    Sampler {
        sampler,
        binding_type: wgpu::SamplerBindingType::Filtering,
        filterable: true,
    }
}

pub fn create_sampler_nearest(device: &wgpu::Device) -> Sampler {
    let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        address_mode_u: wgpu::AddressMode::ClampToEdge,
        address_mode_v: wgpu::AddressMode::ClampToEdge,
        address_mode_w: wgpu::AddressMode::ClampToEdge,
        mag_filter: wgpu::FilterMode::Nearest,
        min_filter: wgpu::FilterMode::Nearest,
        mipmap_filter: wgpu::FilterMode::Nearest,
        ..Default::default()
    });

    Sampler {
        sampler,
        binding_type: wgpu::SamplerBindingType::NonFiltering,
        filterable: false,
    }
}

pub fn load_cubemap(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    mut data: Vec<Cursor<Vec<u8>>>,
    sampler: Sampler,
    format: wgpu::TextureFormat,
    label: Option<&str>,
) -> Result<Texture, String> {
    let mut raw_data: [Vec<u8>; 6] = Default::default();
    let mut last_width = 0;
    for (i, raw_data) in raw_data.iter_mut().enumerate() {
        let img = image::load(data.remove(0), image::ImageFormat::Png)
            .unwrap()
            .flipv()
            .to_rgba8();
        let (width, height) = img.dimensions();
        let raw = img.into_raw();
        *raw_data = raw;
        assert!(width == height, "width must be equal to height in cubemaps");
        if i > 0 {
            assert!(
                width == last_width,
                "sizes of all cubemap sides must be equal"
            );
        }
        last_width = width;
    }

    let data = [
        raw_data[0].as_slice(),
        raw_data[1].as_slice(),
        raw_data[2].as_slice(),
        raw_data[3].as_slice(),
        raw_data[4].as_slice(),
        raw_data[5].as_slice(),
    ];

    load_cubemap_from_bytes(
        device,
        queue,
        &data.concat(),
        last_width,
        sampler,
        format,
        label,
    )
}

//copy of load_texture_from_bytes
pub fn load_cubemap_from_bytes(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    data: &[u8],
    width: u32,
    sampler: Sampler,
    format: wgpu::TextureFormat,
    label: Option<&str>,
) -> Result<Texture, String> {
    let size = wgpu::Extent3d {
        width,
        height: width,
        depth_or_array_layers: 6,
    };

    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label,
        size,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[format],
    });

    let view = texture.create_view(&wgpu::TextureViewDescriptor {
        format: Some(format),
        dimension: Some(wgpu::TextureViewDimension::Cube),
        aspect: wgpu::TextureAspect::default(),
        base_mip_level: 0,
        mip_level_count: Some(1),
        base_array_layer: 0, // this is wrong; setting to 6 gets rid of some errors
        array_layer_count: Some(6),
        label,
    });

    queue.write_texture(
        wgpu::ImageCopyTexture {
            aspect: wgpu::TextureAspect::All,
            texture: &texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
        },
        data,
        wgpu::ImageDataLayout {
            offset: 0,
            bytes_per_row: Some(4 * width),
            rows_per_image: Some(width),
        },
        size,
    );

    Ok(Texture {
        texture: Rc::new(texture),
        view: Rc::new(view),
        sampler: Rc::new(sampler),
        view_dimension: wgpu::TextureViewDimension::Cube,
        label: label.unwrap_or("Unlabeled").to_string(),
    })
}
