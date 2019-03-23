use std::cell::RefCell;
use std::rc::Rc;

use serde_json;

use gfx;
use gfx::traits::FactoryExt;
use gfx::Factory;
use gfx_device_gl;

use vss;

#[test]
fn test_cataract() {
    let width: u16 = 1024;
    let height: u16 = 768;

    let mut factory = initialise_opengl(width as u32, height as u32).factory;

    let (_, source, target) = factory.create_render_target(width, height).unwrap();

    // TODO crashes (because it can't compile the shader?)
    let mut cataract = Cataract::new(&mut factory);

    let values = Rc::new(RefCell::new(serde_json::from_str("{}").unwrap()));

    let view = core::DeviceSource::Rgb { rgba8: source };

    // initialise pass
    cataract.build(
        &mut factory,
        None,
        &DeviceSource,
        &factory.create_sampler_linear(),
        &target,
        &values,
    );

    // render
    let mut encoder: gfx::Encoder<gfx_device_gl::Resources, gfx_device_gl::CommandBuffer> =
        factory.create_command_buffer().into();
    cataract.render(&mut encoder);
}
