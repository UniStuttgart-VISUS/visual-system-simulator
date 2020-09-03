mod cmd;
mod devices;

use vss::*;

use crate::cmd::*;

fn resolve_desktop_devices(config: &Config) -> Option<Box<dyn Device>> {
    match config.device.as_ref() as &str {
        #[cfg(feature = "video")]
        "video" => {
            use crate::devices::*;
            Some(Box::new(AvDevice::new(&config)) as Box<dyn Device>)
        }
        _ => None,
    }
}

pub fn main() {
    let config = cmd_parse();

    let device = config.build(&resolve_desktop_devices).unwrap();

    //XXX: this is a bit out of place, but ok for now.
    device.pipeline().borrow().update_io(&*device);
    device
        .pipeline()
        .borrow()
        .update_params(&*device, &config.parameters);

    let mut done = false;
    while !done {
        device.pipeline().borrow().render(&*device);
        done = device.render();        
    }
}
