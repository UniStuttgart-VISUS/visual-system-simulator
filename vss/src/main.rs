use vss::*;
use std::cell::RefCell;

pub async fn run() {
    let mut parameters = Vec::new();
    let mut value_map = ValueMap::new();
    value_map.insert("flow_id".into(),Value::Number(1.0 as f64));
    parameters.push(RefCell::new(value_map));

    let mut window = Window::new(true, None, parameters, 1).await;

    let input_path = std::path::Path::new("./assets/cubes.rgbd.png");
    // let input_path = std::path::Path::new("./assets/test-calibration.png");
    let mut input_node = UploadRgbBuffer::new(&window);
    input_node.set_flags(RgbInputFlags::from_extension(&input_path));
    input_node.upload_image(load(input_path));
    window.add_node(Box::new(input_node), 0);
    let node = Lens::new(&window);
    window.add_node(Box::new(node), 0);
    // let node = TestNode::new(&window);
    // let node = PeacockCB::new(&window);
    // window.add_node(Box::new(node), 0);
    // let node = Cataract::new(&window);
    // window.add_node(Box::new(node), 0);
    // let node = Retina::new(&window);
    // window.add_node(Box::new(node), 0);
    let node = Display::new(&window);
    window.add_node(Box::new(node), 0);
    window.update_nodes();

    let mut values = ValueMap::new();
    // values.insert("ct_onoff".to_string(), Value::Bool(false));
    // values.insert("ct_blur_factor".to_string(), Value::Number(50.0));
    // values.insert("ct_contrast_factor".to_string(), Value::Number(50.0));
    // values.insert("peacock_cb_onoff".to_string(), Value::Bool(false));
    // values.insert("peacock_cb_strength".to_string(), Value::Number(1.0));
    // values.insert("peacock_cb_type".to_string(), Value::Number(0.0));
    // values.insert("maculardegeneration_veasy".to_string(), Value::Bool(true));
    // values.insert("maculardegeneration_inteasy".to_string(), Value::Number(50.0));
    // values.insert("maculardegeneration_onoff".to_string(), Value::Bool(true));
    // values.insert("colorblindness_type".to_string(), Value::Number(1.0));
    // values.insert("colorblindness_int".to_string(), Value::Number(100.0));
    // values.insert("colorblindness_onoff".to_string(), Value::Bool(true));
    values.insert("presbyopia_onoff".to_string(), Value::Bool(true));
    values.insert("presbyopia_near_point".to_string(), Value::Number(0.0));
    window.set_values(values, 0);

    while !window.poll_events() {}
}

fn main() {
    pollster::block_on(run());
}