mod cmd;
mod nodes;

use vss::*;

use crate::cmd::*;
use crate::nodes::*;

fn node_from_input(input: &str, window: &Window) -> Box<dyn Node> {
    // Resolve input to node.
    if input.ends_with(".png") {
        let mut node = BufferToRgb::new(&window);
        node.enqueue_buffer(load(input));
        Box::new(node)
    } else if input.ends_with(".avi") {
        Box::new(AvToRgb::new(&window))
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
    let mut window = Window::new(config.visible, remote, config.parameters);

    // Resolve input to node.
    window.add_node(node_from_input(&config.ios.first().unwrap().0, &window));

    // Visual system passes.
    let node = Cataract::new(&window);
    window.add_node(Box::new(node));
    let node = Lens::new(&window);
    window.add_node(Box::new(node));
    let node = Retina::new(&window);
    window.add_node(Box::new(node));

    // Inject screenshooting node, if required.
    if !config.ios.first().unwrap().1.is_empty() {
        let mut node = RgbToBuffer::new(&window);
        node.set_output_png(config.ios.first().unwrap().1.clone());
        window.add_node(Box::new(node));
    }

    // Output node.
    let node = RgbToDisplay::new(&window);
    window.add_node(Box::new(node));

    let mut done = false;
    while !done {
        done = window.poll_events();
    }
}
