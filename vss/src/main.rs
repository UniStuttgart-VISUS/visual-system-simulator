use vss::*;
use std::cell::RefCell;

pub async fn run() {
    let mut parameters = Vec::new();
    let mut value_map = ValueMap::new();
    value_map.insert("flow_id".into(),Value::Number(1.0 as f64));
    parameters.push(RefCell::new(value_map));

    let mut window = Window::new(true, None, parameters, 1).await;

    let input_path = std::path::Path::new("./assets/flowers.png");
    let mut input_node = UploadRgbBuffer::new(&window);
    input_node.upload_image(load(input_path));
    window.add_node(Box::new(input_node), 0);
    let node = TestNode::new(&window);
    // let node = Display::new(&window);
    window.add_node(Box::new(node), 0);
    window.update_nodes();

    while !window.poll_events() {}
}

fn main() {
    pollster::block_on(run());
}