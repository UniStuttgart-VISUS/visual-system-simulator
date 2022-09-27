use vss::*;
use std::cell::RefCell;

pub async fn run() {
    let mut parameters = Vec::new();
    let mut value_map = ValueMap::new();
    value_map.insert("flow_id".into(),Value::Number(1.0 as f64));
    parameters.push(RefCell::new(value_map));

    let mut window = Window::new(true, None, parameters, 1).await;

    let node = TestNode::new(&window);
    window.add_node(Box::new(node), 0);

    while !window.poll_events() {}
}

fn main() {
    pollster::block_on(run());
}