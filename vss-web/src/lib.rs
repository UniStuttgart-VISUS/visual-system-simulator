use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn run() {
    use web_sys::console;

    console::log_1(&"Hello World!".into());
}
