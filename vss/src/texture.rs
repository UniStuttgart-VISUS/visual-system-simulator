use wgpu::{TextureView, RenderPassColorAttachment, RenderPassDepthStencilAttachment};

use crate::*;
// use gfx::Factory;
// use gfx_device_gl::CommandBuffer;
// use gfx_device_gl::Resources;
// use std::io::Cursor;
use core::num::NonZeroU32;
use std::{rc::Rc};

pub struct Texture {
    //TODO maybe use RefCell
    pub texture: Rc<wgpu::Texture>,
    pub view: Rc<wgpu::TextureView>,
    pub width: u32,
    pub height: u32,
}

pub struct RenderTexture{
    pub texture: Option<Rc<wgpu::Texture>>,
    pub view: Rc<wgpu::TextureView>,
    pub width: u32,
    pub height: u32,
}

pub struct Sampler {
    pub sampler: wgpu::Sampler,
}

impl Texture{
    pub fn create_bind_group(&self, device: &wgpu::Device, sampler: &Sampler)-> (wgpu::BindGroupLayout, wgpu::BindGroup){
        create_texture_bind_group(device, self.view.as_ref(), sampler)
    }
    
    pub fn clone(&self) -> Texture{
        Texture{
            texture: self.texture.clone(),
            view: self.view.clone(),
            width: self.width,
            height: self.height,
        }
    }
}

impl RenderTexture{
    pub fn create_bind_group(&self, device: &wgpu::Device, sampler: &Sampler)-> (wgpu::BindGroupLayout, wgpu::BindGroup){
        create_texture_bind_group(device, self.view.as_ref(), sampler)
    }

    pub fn clone(&self) -> RenderTexture{
        RenderTexture{
            texture: self.texture.clone(),
            view: self.view.clone(),
            width: self.width,
            height: self.height,
        }
    }

    pub fn as_texture(&self) -> Texture{
        Texture{
            texture: self.texture.clone().unwrap(),
            view: self.view.clone(),
            width: self.width,
            height: self.height,
        }
    }

    pub fn to_color_attachment(&self) -> Option<RenderPassColorAttachment>{
        Some(wgpu::RenderPassColorAttachment {
            view: self.view.as_ref(),
            resolve_target: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Clear(wgpu::Color {
                    r: 0.5,
                    g: 0.5,
                    b: 0.5,
                    a: 1.0,
                }),
                store: true,
            },
        })
    }

    pub fn to_depth_attachment(&self) -> Option<RenderPassDepthStencilAttachment>{
        Some(wgpu::RenderPassDepthStencilAttachment {
            view: self.view.as_ref(),
            depth_ops: Some(wgpu::Operations {
                load: wgpu::LoadOp::Clear(1.0),
                store: true,
            }),
            stencil_ops: None,
        })
    }
}

fn create_texture_bind_group(device: &wgpu::Device, view: &TextureView, sampler: &Sampler) 
    -> (wgpu::BindGroupLayout, wgpu::BindGroup)
    {
    let layout =
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    multisampled: false,
                    view_dimension: wgpu::TextureViewDimension::D2,
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
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
                resource: wgpu::BindingResource::TextureView(view),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::Sampler(&sampler.sampler),
            },
        ],
        label: Some("texture_bind_group"),
    });

    (layout, bind_group)
}

pub fn create_textures_bind_group(device: &wgpu::Device, textures: &[&Texture], sampler: &Sampler) 
    -> (wgpu::BindGroupLayout, wgpu::BindGroup)
    {

    let alt_sampler = create_sampler_nearest(device).unwrap(); //TODO WGPU move this to texture ?
    
    let mut layout_entries: Vec::<wgpu::BindGroupLayoutEntry> = vec![wgpu::BindGroupLayoutEntry {
        binding: 0,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::NonFiltering), //TODO make this ::Filtering depending on Texture
        count: None,
    }];
    layout_entries.extend(textures.iter().enumerate().map(
        |(i, _)|
        wgpu::BindGroupLayoutEntry{
            binding: i as u32 + 1,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Texture {
                multisampled: false,
                view_dimension: wgpu::TextureViewDimension::D2,
                sample_type: wgpu::TextureSampleType::Float { filterable: false }, //TODO make this true depending on Texture
            },
            count: None,
        }
    ));

    let layout =
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        entries: &layout_entries,
        label: Some("textures_bind_group_layout"),
    });

    let mut group_entries: Vec::<wgpu::BindGroupEntry> = vec![wgpu::BindGroupEntry {
        binding: 0,
        resource: wgpu::BindingResource::Sampler(&alt_sampler.sampler),
    }];
    group_entries.extend(textures.iter().enumerate().map(
        |(i, t)|
        wgpu::BindGroupEntry {
            binding: i as u32 + 1,
            resource: wgpu::BindingResource::TextureView(&t.view),
        }
    ));

    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        layout: &layout,
        entries: &group_entries,
        label: Some("textures_bind_group"),
    });

    (layout, bind_group)
}

///
/// Can be used to replace parts of or a whole texture.
///
/// # Example
///
/// To replace 64x64 pixels in the lower left of the texture with 0xff00ff, do:
///
/// ```rust,ignore
/// let arr = vec![0xffff00ff; 64*64];
/// let data = gfx::memory::cast_slice(&arr);
/// let size = [64, 64];
/// let offset = [0, 0];
/// update_texture(encoder, &self.texture, size, offset, data);
/// ```
///
pub fn update_texture(
    queue: &wgpu::Queue,
    texture: &Texture,
    size: [u32; 2],
    // offset: [u16; 2],
    raw_data: &[u8],
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
            origin: wgpu::Origin3d::ZERO,
        },
        raw_data,
        wgpu::ImageDataLayout {
            offset: 0,
            bytes_per_row: NonZeroU32::new(4 * size[0]),
            rows_per_image: NonZeroU32::new(size[1]),
        },
        texture_size,
    );
}

// pub fn load_texture(
//     factory: &mut gfx_device_gl::Factory,
//     data: Cursor<Vec<u8>>,
// ) -> Result<
//     (
//         gfx::handle::Texture<Resources, gfx::format::R8_G8_B8_A8>,
//         gfx::handle::ShaderResourceView<Resources, [f32; 4]>,
//     ),
//     String,
// > {
//     let img = image::load(data, image::ImageFormat::Png)
//         .unwrap()
//         .flipv()
//         .to_rgba8();
//     let (width, height) = img.dimensions();
//     let data = img.into_raw();

//     load_texture_from_bytes(factory, &data, width, height)
// }

///
/// Load bytes as texture into GPU
///
/// # Arguments
///
/// - `factory` - factory to generate commands for opengl command buffer
/// - `data` - raw image data
/// - `width` - width of the requested texture
/// - `height` - height of the requested texture
///
/// # Return
///
/// Created Texture and shader RessourceView
///
pub fn load_texture_from_bytes(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    data: &[u8],
    width: u32,
    height: u32,
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
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
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
            bytes_per_row: NonZeroU32::new(4 * width),
            rows_per_image: NonZeroU32::new(height),
        },
        size,
    );

    Ok(Texture {
        texture: Rc::new(texture),
        view: Rc::new(view),
        width,
        height,
    })
}

pub fn placeholder_texture(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    label: Option<&str>,
) -> Result<Texture, String> {
    load_texture_from_bytes(device, queue, &[0; 4], 1, 1, label)
}

pub fn create_sampler_linear(device: &wgpu::Device) -> Result<Sampler, String>{
    let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        address_mode_u: wgpu::AddressMode::ClampToEdge,
        address_mode_v: wgpu::AddressMode::ClampToEdge,
        address_mode_w: wgpu::AddressMode::ClampToEdge,
        mag_filter: wgpu::FilterMode::Linear,
        min_filter: wgpu::FilterMode::Linear,
        mipmap_filter: wgpu::FilterMode::Linear,
        ..Default::default()
    });

    Ok(Sampler {
        sampler,
    })
}

pub fn create_sampler_nearest(device: &wgpu::Device) -> Result<Sampler, String>{
    let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        address_mode_u: wgpu::AddressMode::ClampToEdge,
        address_mode_v: wgpu::AddressMode::ClampToEdge,
        address_mode_w: wgpu::AddressMode::ClampToEdge,
        mag_filter: wgpu::FilterMode::Nearest,
        min_filter: wgpu::FilterMode::Nearest,
        mipmap_filter: wgpu::FilterMode::Nearest,
        ..Default::default()
    });

    Ok(Sampler {
        sampler,
    })
}

// pub fn load_cubemap(
//     factory: &mut gfx_device_gl::Factory,
//     mut data: Vec<Cursor<Vec<u8>>>,
// ) -> Result<
//     (
//         gfx::handle::Texture<Resources, gfx::format::R8_G8_B8_A8>,
//         gfx::handle::ShaderResourceView<Resources, [f32; 4]>,
//     ),
//     String,
// > {
//     let mut raw_data: [Vec<u8>; 6] = Default::default();
//     let mut last_width = 0;
//     for i in 0..6 {
//         let img = image::load(data.remove(0), image::ImageFormat::Png)
//             .unwrap()
//             .flipv()
//             .to_rgba8();
//         let (width, height) = img.dimensions();
//         let raw = img.into_raw();
//         raw_data[i] = raw;
//         assert!(width == height, "width must be equal to height in cubemaps");
//         if i > 0 {
//             assert!(width == last_width, "sizes of all cubemap sides must be equal");
//         }
//         last_width = width;
//     }

//     load_cubemap_from_bytes(factory, &[raw_data[0].as_slice(), raw_data[1].as_slice(), raw_data[2].as_slice(), raw_data[3].as_slice(), raw_data[4].as_slice(), raw_data[5].as_slice()], last_width)
// }

//copy of load_texture_from_bytes
// pub fn load_cubemap_from_bytes(
//     factory: &mut gfx_device_gl::Factory,
//     data: &[&[u8];6],
//     width: u32,
// ) -> Result<
//     (
//         gfx::handle::Texture<Resources, gfx::format::R8_G8_B8_A8>,
//         gfx::handle::ShaderResourceView<Resources, [f32; 4]>,
//     ),
//     String,
// > {
//     let kind = texture::Kind::Cube(
//         width as texture::Size,
//     );

//     // inspired by https://github.com/PistonDevelopers/gfx_texture/blob/master/src/lib.rs#L157-L178
//     use gfx::memory::Typed;
//     use gfx::memory::Usage;
//     use gfx::{format, texture};

//     let surface = gfx::format::SurfaceType::R8_G8_B8_A8;
//     let desc = texture::Info {
//         kind,
//         levels: 1 as texture::Level,
//         format: surface,
//         bind: gfx::memory::Bind::all(),
//         usage: Usage::Dynamic,
//     };

//     let cty = gfx::format::ChannelType::Unorm;
//     let raw = factory
//         .create_texture_raw(
//             desc,
//             Some(cty),
//             Some((data, gfx::texture::Mipmap::Allocated)),
//         )
//         .unwrap();
//     let levels = (0, raw.get_info().levels - 1);
//     let tex = Typed::new(raw);
//     let view = factory
//         .view_texture_as_shader_resource::<ColorFormat>(&tex, levels, format::Swizzle::new())
//         .unwrap();
//     Ok((tex, view))
// }


///
/// Load bytes as texture into GPU
///
/// # Arguments
///
/// - `factory` - factory to generate commands for opengl command buffer
/// - `data` - raw image data
/// - `width` - width of the requested texture
/// - `height` - height of the requested texture
///
/// # Return
///
/// Created Texture and shader RessourceView
///
// pub fn load_highp_texture_from_bytes(
//     factory: &mut gfx_device_gl::Factory,
//     data: &[u8],
//     width: u32,
//     height: u32,
// ) -> Result<
//     (
//         gfx::handle::Texture<Resources, gfx::format::R32_G32_B32_A32>,
//         gfx::handle::ShaderResourceView<Resources, [f32; 4]>,
//     ),
//     String,
// > {
//     let kind = texture::Kind::D2(
//         width as texture::Size,
//         height as texture::Size,
//         texture::AaMode::Single,
//     );

//     // inspired by https://github.com/PistonDevelopers/gfx_texture/blob/master/src/lib.rs#L157-L178
//     use gfx::memory::Typed;
//     use gfx::memory::Usage;
//     use gfx::{format, texture};

//     let surface = gfx::format::SurfaceType::R32_G32_B32;
//     let desc = texture::Info {
//         kind,
//         levels: 1 as texture::Level,
//         format: surface,
//         bind: gfx::memory::Bind::all(),
//         usage: Usage::Dynamic,
//     };

//     let cty = gfx::format::ChannelType::Float;
//     let raw = factory
//         .create_texture_raw(
//             desc,
//             Some(cty),
//             Some((&[&data], gfx::texture::Mipmap::Allocated)),
//         )
//         .unwrap();
//     let levels = (0, raw.get_info().levels - 1);
//     let tex = Typed::new(raw);
//     let view = factory
//         .view_texture_as_shader_resource::<(gfx::format::R32_G32_B32_A32, gfx::format::Float)>(&tex, levels, format::Swizzle::new())
//         .unwrap();
//     Ok((tex, view))
// }

// pub fn update_single_channel_texture(
//     encoder: &mut gfx::Encoder<Resources, CommandBuffer>,
//     texture: &gfx::handle::Texture<Resources, gfx::format::R8>,
//     size: [u16; 2],
//     offset: [u16; 2],
//     raw_data: &[u8],
// ) {
//     let img_info = gfx::texture::ImageInfoCommon {
//         xoffset: offset[0],
//         yoffset: offset[1],
//         zoffset: 0,
//         width: size[0],
//         height: size[1],
//         depth: 0,
//         format: (),
//         mipmap: 0,
//     };

//     let data = gfx::memory::cast_slice(&raw_data);
//     let _msg = encoder.update_texture::<gfx::format::R8, (gfx::format::R8, gfx::format::Unorm)>(
//         texture, None, img_info, data,
//     );
// }

// pub fn load_single_channel_texture_from_bytes(
//     factory: &mut gfx_device_gl::Factory,
//     data: &[u8],
//     width: u32,
//     height: u32,
// ) -> Result<
//     (
//         gfx::handle::Texture<Resources, gfx::format::R8>,
//         gfx::handle::ShaderResourceView<Resources, f32>,
//     ),
//     String,
// > {
//     let kind = texture::Kind::D2(
//         width as texture::Size,
//         height as texture::Size,
//         texture::AaMode::Single,
//     );

//     // inspired by https://github.com/PistonDevelopers/gfx_texture/blob/master/src/lib.rs#L157-L178
//     use gfx::memory::Typed;
//     use gfx::memory::Usage;
//     use gfx::{format, texture};

//     let surface = gfx::format::SurfaceType::R8;
//     let desc = texture::Info {
//         kind,
//         levels: 1 as texture::Level,
//         format: surface,
//         bind: gfx::memory::Bind::all(),
//         usage: Usage::Dynamic,
//     };

//     let cty = gfx::format::ChannelType::Unorm;
//     let raw = factory
//         .create_texture_raw(
//             desc,
//             Some(cty),
//             Some((&[&data], gfx::texture::Mipmap::Allocated)),
//         )
//         .unwrap();
//     let levels = (0, raw.get_info().levels - 1);
//     let tex = Typed::new(raw);
//     let view = factory
//         .view_texture_as_shader_resource::<(gfx::format::R8, gfx::format::Unorm)>(
//             &tex,
//             levels,
//             format::Swizzle::new(),
//         )
//         .unwrap();
//     Ok((tex, view))
// }

// pub fn load_highres_normalmap(
//     factory: &mut gfx_device_gl::Factory,
//     data: Cursor<Vec<u8>>,
// ) -> Result<
//     (
//         gfx::handle::Texture<Resources, gfx::format::R32_G32_B32_A32>,
//         gfx::handle::ShaderResourceView<Resources, [f32; 4]>,
//     ),
//     String,
// > {
//     let img = image::load(data, image::ImageFormat::Png)
//         .unwrap()
//         .flipv()
//         .to_rgba8();
//     let (width, height) = img.dimensions();
//     let data_raw = img.into_raw();

//     let mut data_float = Vec::new();

//     for i in 0..(data_raw.len() / 4) {
//         let n = ((data_raw[i * 4 + 3] as u32) << 24)
//             | ((data_raw[i * 4] as u32) << 16)
//             | ((data_raw[i * 4 + 1] as u32) << 8)
//             | (data_raw[i * 4 + 2] as u32);
//         data_float.push((n as f32) / (<u32>::max_value() as f32));
//     }

//     let data = unsafe {
//         std::slice::from_raw_parts(data_float.as_mut_ptr() as *const u8, data_float.len() * 4)
//     };

//     let kind = texture::Kind::D2(
//         (width / 3) as texture::Size,
//         height as texture::Size,
//         texture::AaMode::Single,
//     );

//     // inspired by https://github.com/PistonDevelopers/gfx_texture/blob/master/src/lib.rs#L157-L178
//     use gfx::memory::Typed;
//     use gfx::memory::Usage;
//     use gfx::{format, texture};

//     let surface = gfx::format::SurfaceType::R32_G32_B32;
//     let desc = texture::Info {
//         kind,
//         levels: 1 as texture::Level,
//         format: surface,
//         bind: gfx::memory::Bind::all(),
//         usage: Usage::Dynamic,
//     };

//     let cty = gfx::format::ChannelType::Float;
//     let raw = factory
//         .create_texture_raw(
//             desc,
//             Some(cty),
//             Some((&[data], gfx::texture::Mipmap::Allocated)),
//         )
//         .unwrap();
//     let levels = (0, raw.get_info().levels - 1);
//     let tex = Typed::new(raw);
//     let view = factory
//         .view_texture_as_shader_resource::<(gfx::format::R32_G32_B32_A32, gfx::format::Float)>(
//             &tex,
//             levels,
//             format::Swizzle::new(),
//         )
//         .unwrap();
//     Ok((tex, view))
// }

/// Creates a texture that can be read from in shaders (view) and rendered to (render target).
pub fn create_texture_render_target(
    device: &wgpu::Device,
    width: u32,
    height: u32,
    format: wgpu::TextureFormat,
    label: Option<&str>,
) -> RenderTexture{
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
        format: format,
        usage: wgpu::TextureUsages::TEXTURE_BINDING |
               wgpu::TextureUsages::RENDER_ATTACHMENT |
               wgpu::TextureUsages::COPY_SRC,
    });

    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

    RenderTexture{
        texture: Some(Rc::new(texture)),
        view: Rc::new(view),
        width,
        height,
    }
}

pub fn placeholder_render_texture(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    label: Option<&str>,
) -> RenderTexture{
    create_texture_render_target(device, 1, 1, format, label)
}

// pub fn texture_from_id_and_size<T>(
//     texture_id: gfx_gl::types::GLuint,
//     width: u32,
//     height: u32,
// ) -> gfx_core::handle::Texture<gfx_device_gl::Resources, <T as gfx::format::Formatted>::Surface>
// where
//     T: gfx::format::TextureFormat + gfx::format::RenderFormat,
// {
//     use gfx_core::handle::Producer;
//     let mut temp: gfx_core::handle::Manager<gfx_device_gl::Resources> =
//         gfx_core::handle::Manager::new();
//     let raw_texture = temp.make_texture(
//         gfx_device_gl::NewTexture::Texture(texture_id),
//         gfx_core::texture::Info {
//             levels: 1,
//             kind: gfx_core::texture::Kind::D2(
//                 width as u16,
//                 height as u16,
//                 gfx_core::texture::AaMode::Single,
//             ),
//             format: gfx_core::format::SurfaceType::R8_G8_B8_A8,
//             bind: gfx_core::memory::Bind::RENDER_TARGET | gfx_core::memory::Bind::TRANSFER_SRC,
//             usage: gfx_core::memory::Usage::Data,
//         },
//     );
//     use crate::gfx::memory::Typed;
//     gfx::handle::Texture::new(raw_texture)
// }

// pub fn depth_texture_from_id_and_size<T>(
//     texture_id: gfx_gl::types::GLuint,
//     width: u32,
//     height: u32,
// ) -> gfx_core::handle::Texture<gfx_device_gl::Resources, <T as gfx::format::Formatted>::Surface>
// where
//     T: gfx::format::TextureFormat + gfx::format::DepthFormat,
// {
//     use gfx_core::handle::Producer;
//     let mut temp: gfx_core::handle::Manager<gfx_device_gl::Resources> =
//         gfx_core::handle::Manager::new();
//     let raw_texture = temp.make_texture(
//         gfx_device_gl::NewTexture::Texture(texture_id),
//         gfx_core::texture::Info {
//             levels: 1,
//             kind: gfx_core::texture::Kind::D2(
//                 width as u16,
//                 height as u16,
//                 gfx_core::texture::AaMode::Single,
//             ),
//             format: gfx_core::format::SurfaceType::D24_S8,
//             bind: gfx_core::memory::Bind::DEPTH_STENCIL | gfx_core::memory::Bind::TRANSFER_SRC,
//             usage: gfx_core::memory::Usage::Data,
//         },
//     );
//     use crate::gfx::memory::Typed;
//     gfx::handle::Texture::new(raw_texture)
// }
