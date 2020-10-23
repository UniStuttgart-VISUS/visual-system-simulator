use crate::*;
use gfx::Factory;
use gfx_device_gl::CommandBuffer;
use gfx_device_gl::Resources;
use std::io::Cursor;

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
    encoder: &mut gfx::Encoder<Resources, CommandBuffer>,
    texture: &gfx::handle::Texture<Resources, gfx::format::R8_G8_B8_A8>,
    size: [u16; 2],
    offset: [u16; 2],
    raw_data: &[u8],
) {
    let img_info = gfx::texture::ImageInfoCommon {
        xoffset: offset[0],
        yoffset: offset[1],
        zoffset: 0,
        width: size[0],
        height: size[1],
        depth: 0,
        format: (),
        mipmap: 0,
    };

    let data = gfx::memory::cast_slice(&raw_data);
    let _msg = encoder
        .update_texture::<gfx::format::R8_G8_B8_A8, ColorFormat>(texture, None, img_info, data);
}

pub fn load_texture(
    factory: &mut gfx_device_gl::Factory,
    data: Cursor<Vec<u8>>,
) -> Result<
    (
        gfx::handle::Texture<Resources, gfx::format::R8_G8_B8_A8>,
        gfx::handle::ShaderResourceView<Resources, [f32; 4]>,
    ),
    String,
> {
    let img = image::load(data, image::ImageFormat::Png)
        .unwrap()
        .flipv()
        .to_rgba();
    let (width, height) = img.dimensions();
    let data = img.into_raw();

    load_texture_from_bytes(factory, &data, width, height)
}

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
    factory: &mut gfx_device_gl::Factory,
    data: &[u8],
    width: u32,
    height: u32,
) -> Result<
    (
        gfx::handle::Texture<Resources, gfx::format::R8_G8_B8_A8>,
        gfx::handle::ShaderResourceView<Resources, [f32; 4]>,
    ),
    String,
> {
    let kind = texture::Kind::D2(
        width as texture::Size,
        height as texture::Size,
        texture::AaMode::Single,
    );

    // inspired by https://github.com/PistonDevelopers/gfx_texture/blob/master/src/lib.rs#L157-L178
    use gfx::memory::Typed;
    use gfx::memory::Usage;
    use gfx::{format, texture};

    let surface = gfx::format::SurfaceType::R8_G8_B8_A8;
    let desc = texture::Info {
        kind,
        levels: 1 as texture::Level,
        format: surface,
        bind: gfx::memory::Bind::all(),
        usage: Usage::Dynamic,
    };

    let cty = gfx::format::ChannelType::Unorm;
    let raw = factory
        .create_texture_raw(
            desc,
            Some(cty),
            Some((&[&data], gfx::texture::Mipmap::Allocated)),
        )
        .unwrap();
    let levels = (0, raw.get_info().levels - 1);
    let tex = Typed::new(raw);
    let view = factory
        .view_texture_as_shader_resource::<ColorFormat>(&tex, levels, format::Swizzle::new())
        .unwrap();
    Ok((tex, view))
}

pub fn update_single_channel_texture(
    encoder: &mut gfx::Encoder<Resources, CommandBuffer>,
    texture: &gfx::handle::Texture<Resources, gfx::format::R8>,
    size: [u16; 2],
    offset: [u16; 2],
    raw_data: &[u8],
) {
    let img_info = gfx::texture::ImageInfoCommon {
        xoffset: offset[0],
        yoffset: offset[1],
        zoffset: 0,
        width: size[0],
        height: size[1],
        depth: 0,
        format: (),
        mipmap: 0,
    };

    let data = gfx::memory::cast_slice(&raw_data);
    let _msg = encoder.update_texture::<gfx::format::R8, (gfx::format::R8, gfx::format::Unorm)>(
        texture, None, img_info, data,
    );
}

pub fn load_single_channel_texture_from_bytes(
    factory: &mut gfx_device_gl::Factory,
    data: &[u8],
    width: u32,
    height: u32,
) -> Result<
    (
        gfx::handle::Texture<Resources, gfx::format::R8>,
        gfx::handle::ShaderResourceView<Resources, f32>,
    ),
    String,
> {
    let kind = texture::Kind::D2(
        width as texture::Size,
        height as texture::Size,
        texture::AaMode::Single,
    );

    // inspired by https://github.com/PistonDevelopers/gfx_texture/blob/master/src/lib.rs#L157-L178
    use gfx::memory::Typed;
    use gfx::memory::Usage;
    use gfx::{format, texture};

    let surface = gfx::format::SurfaceType::R8;
    let desc = texture::Info {
        kind,
        levels: 1 as texture::Level,
        format: surface,
        bind: gfx::memory::Bind::all(),
        usage: Usage::Dynamic,
    };

    let cty = gfx::format::ChannelType::Unorm;
    let raw = factory
        .create_texture_raw(
            desc,
            Some(cty),
            Some((&[&data], gfx::texture::Mipmap::Allocated)),
        )
        .unwrap();
    let levels = (0, raw.get_info().levels - 1);
    let tex = Typed::new(raw);
    let view = factory
        .view_texture_as_shader_resource::<(gfx::format::R8, gfx::format::Unorm)>(
            &tex,
            levels,
            format::Swizzle::new(),
        )
        .unwrap();
    Ok((tex, view))
}

pub fn load_highres_normalmap(
    factory: &mut gfx_device_gl::Factory,
    data: Cursor<Vec<u8>>,
) -> Result<
    (
        gfx::handle::Texture<Resources, gfx::format::R32_G32_B32_A32>,
        gfx::handle::ShaderResourceView<Resources, [f32; 4]>,
    ),
    String,
> {
    let img = image::load(data, image::ImageFormat::Png)
        .unwrap()
        .flipv()
        .to_rgba();
    let (width, height) = img.dimensions();
    let data_raw = img.into_raw();

    let mut data_float = Vec::new();

    for i in 0..(data_raw.len() / 4) {
        let n = ((data_raw[i * 4 + 3] as u32) << 24)
            | ((data_raw[i * 4] as u32) << 16)
            | ((data_raw[i * 4 + 1] as u32) << 8)
            | (data_raw[i * 4 + 2] as u32);
        data_float.push((n as f32) / (<u32>::max_value() as f32));
    }

    let data = unsafe {
        std::slice::from_raw_parts(data_float.as_mut_ptr() as *const u8, data_float.len() * 4)
    };

    let kind = texture::Kind::D2(
        (width / 3) as texture::Size,
        height as texture::Size,
        texture::AaMode::Single,
    );

    // inspired by https://github.com/PistonDevelopers/gfx_texture/blob/master/src/lib.rs#L157-L178
    use gfx::memory::Typed;
    use gfx::memory::Usage;
    use gfx::{format, texture};

    let surface = gfx::format::SurfaceType::R32_G32_B32;
    let desc = texture::Info {
        kind,
        levels: 1 as texture::Level,
        format: surface,
        bind: gfx::memory::Bind::all(),
        usage: Usage::Dynamic,
    };

    let cty = gfx::format::ChannelType::Float;
    let raw = factory
        .create_texture_raw(
            desc,
            Some(cty),
            Some((&[data], gfx::texture::Mipmap::Allocated)),
        )
        .unwrap();
    let levels = (0, raw.get_info().levels - 1);
    let tex = Typed::new(raw);
    let view = factory
        .view_texture_as_shader_resource::<(gfx::format::R32_G32_B32_A32, gfx::format::Float)>(
            &tex,
            levels,
            format::Swizzle::new(),
        )
        .unwrap();
    Ok((tex, view))
}

/// Creates a texture that can be read from in shaders (view) and rendered to (render target).
pub fn create_texture_render_target<T>(
    factory: &mut gfx_device_gl::Factory,
    width: u32,
    height: u32,
) -> (
    gfx::handle::RenderTargetView<gfx_device_gl::Resources, T>,
    gfx::handle::ShaderResourceView<gfx_device_gl::Resources, <T as gfx::format::Formatted>::View>,
)
where
    T: gfx::format::TextureFormat + gfx::format::RenderFormat,
{
    let texture = factory
        .create_texture(
            gfx::texture::Kind::D2(
                width as u16,
                height as u16,
                gfx::texture::AaMode::Single,
            ),
            1,
            gfx::memory::Bind::RENDER_TARGET | gfx::memory::Bind::SHADER_RESOURCE | gfx::memory::Bind::TRANSFER_SRC,
            gfx::memory::Usage::Dynamic,
            Some( <<T as gfx::format::Formatted>::Channel as gfx::format::ChannelTyped>::get_channel_type() ),
        )
        .unwrap();
    let target_view = factory
        .view_texture_as_shader_resource::<T>(&texture, (0, 0), gfx::format::Swizzle::new())
        .unwrap();
    let target = factory
        .view_texture_as_render_target::<T>(&texture, 0, None)
        .unwrap();
    (target, target_view)
}

pub fn texture_from_id_and_size<T>(
    texture_id: gfx_gl::types::GLuint,
    width: u32,
    height: u32,
) -> gfx_core::handle::Texture<gfx_device_gl::Resources, <T as gfx::format::Formatted>::Surface>
where
    T: gfx::format::TextureFormat + gfx::format::RenderFormat,
{
    use gfx_core::handle::Producer;
    let mut temp: gfx_core::handle::Manager<gfx_device_gl::Resources> =
        gfx_core::handle::Manager::new();
    let raw_texture = temp.make_texture(
        gfx_device_gl::NewTexture::Texture(texture_id),
        gfx_core::texture::Info {
            levels: 1,
            kind: gfx_core::texture::Kind::D2(
                width as u16,
                height as u16,
                gfx_core::texture::AaMode::Single,
            ),
            format: gfx_core::format::SurfaceType::R8_G8_B8_A8,
            bind: gfx_core::memory::Bind::RENDER_TARGET | gfx_core::memory::Bind::TRANSFER_SRC,
            usage: gfx_core::memory::Usage::Data,
        },
    );
    use crate::gfx::memory::Typed;
    gfx::handle::Texture::new(raw_texture)
}
