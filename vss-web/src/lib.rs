#![cfg(target_arch = "wasm32")]
use vss::*;
use vss_winit::*;
use wasm_bindgen::prelude::*;

fn build_flow(surface: &mut Surface) {
    //TODO: use a proper set of nodes.

    // TODO: input node.

    // Visual system passes.
    let node = Cataract::new(surface);
    surface.add_node(Box::new(node), 0);
    // let node = Lens::new(surface);
    // surface.add_node(Box::new(node), 0);
    let node = Retina::new(surface);
    surface.add_node(Box::new(node), 0);
    let node = PeacockCB::new(surface);
    surface.add_node(Box::new(node), 0);

    // Display node.
    let mut node = Display::new(surface);
    node.set_output_scale(OutputScale::Fill);
    surface.add_node(Box::new(node), 0);

    surface.negociate_slots();
}

#[wasm_bindgen]
pub fn append_and_run(parent_id: &str) {
    std::panic::set_hook(Box::new(console_error_panic_hook::hook));
    console_log::init().expect("Could not initialize logger");

    let mut window_surface = WindowSurface::new(true, 1, None);

    use winit::platform::web::WindowExtWebSys;
    let document = web_sys::window()
        .unwrap()
        .document()
        .expect("Cannot access document");
    let el = document
        .get_element_by_id(parent_id)
        .expect("Cannot find parent element");
    el.append_child(&web_sys::Element::from(window_surface.window().canvas()))
        .expect("Cannot append canvas element");

    wasm_bindgen_futures::spawn_local(window_surface.run_and_exit(
        |surface| {
            build_flow(surface);
        },
        || true,
    ));
}
