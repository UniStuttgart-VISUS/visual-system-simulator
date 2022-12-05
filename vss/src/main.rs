use vss::*;
use std::cell::RefCell;

pub async fn run() {
    let mut parameters = Vec::new();
    let mut value_map = ValueMap::new();
    value_map.insert("flow_id".into(),Value::Number(1.0 as f64));
    parameters.push(RefCell::new(value_map));

    let mut window = Window::new(true, None, parameters, 1).await;

    let input_path = std::path::Path::new("./assets/test-calibration.png");
    let mut input_node = UploadRgbBuffer::new(&window);
    input_node.upload_image(load(input_path));
    window.add_node(Box::new(input_node), 0);
    // let node = TestNode::new(&window);
    let node = PeacockCB::new(&window);
    window.add_node(Box::new(node), 0);
    let node = Cataract::new(&window);
    window.add_node(Box::new(node), 0);
    let node = Display::new(&window);
    window.add_node(Box::new(node), 0);
    window.update_nodes();

    window.set_value("ct_onoff".to_string(), Value::Bool(true), 0);
    window.set_value("ct_blur_factor".to_string(), Value::Number(50.0), 0);
    window.set_value("ct_contrast_factor".to_string(), Value::Number(50.0), 0);
    window.set_value("peacock_cb_onoff".to_string(), Value::Bool(true), 0);
    window.set_value("peacock_cb_strength".to_string(), Value::Number(1.0), 0);
    window.set_value("peacock_cb_type".to_string(), Value::Number(0.0), 0);

    while !window.poll_events() {}
}

fn main() {
    pollster::block_on(run());
}