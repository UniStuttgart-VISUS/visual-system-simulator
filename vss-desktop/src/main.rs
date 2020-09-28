mod cmd;
mod nodes;

use vss::*;

use crate::cmd::*;
use crate::nodes::*;

fn node_from_input(input: &str, window: &Window) -> Box<dyn Node> {
    // Resolve input to node.
    if input.ends_with(".png") {
        let mut node = BufferToRgb::new(&mut window.factory().borrow_mut());
        node.enqueue_buffer(load(input));
        Box::new(node)
    } else if input.ends_with(".avi") {
        Box::new(AvToRgb::new(&mut window.factory().borrow_mut()))
    } else {
        panic!("Unknown file extension");
    }
}

pub fn main() {
    let config = cmd_parse();

    let remote = if config.port != 0 {
        Some(Remote::new(config.port))
    } else {
        None
    };
    let mut window = Window::new(
        config.ios.first().unwrap().1.is_empty(),
        remote,
        config.parameters,
    );

    // Resolve input to node.
    window.add_node(node_from_input(&config.ios.first().unwrap().0, &window));

    // Visual system passes.
    let node = Cataract::new(&mut window.factory().borrow_mut());
    window.add_node(Box::new(node));
    let node = Lens::new(&mut window.factory().borrow_mut());
    window.add_node(Box::new(node));
    let node = Retina::new(&mut window.factory().borrow_mut());
    window.add_node(Box::new(node));

    // Resolve output to node.
    if !config.ios.first().unwrap().1.is_empty() {
        //XXX RbgToBuffer;
    } else {
        let node = RgbToDisplay::new(&mut window.factory().borrow_mut());
        window.add_node(Box::new(node));
    }

    let mut done = false;
    while !done {
        done = window.poll_events();
    }
}
