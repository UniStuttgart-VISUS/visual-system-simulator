#![cfg(target_arch = "wasm32")]
use std::sync::mpsc::{self, Receiver, SyncSender};
use vss::{RgbBuffer, UploadRgbBuffer, *};
use vss_winit::*;
use wasm_bindgen::prelude::*;

struct UploadStream {
    upload: UploadRgbBuffer,
    frame_receiver: Receiver<RgbBuffer>,
}

impl UploadStream {
    fn new(surface: &Surface, frame_receiver: Receiver<RgbBuffer>) -> Self {
        UploadStream {
            upload: UploadRgbBuffer::new(surface),
            frame_receiver,
        }
    }
}

impl Node for UploadStream {
    fn name(&self) -> &'static str {
        "UploadStream"
    }

    fn negociate_slots(
        &mut self,
        surface: &Surface,
        slots: NodeSlots,
        original_image: &mut Option<Texture>,
    ) -> NodeSlots {
        self.upload.negociate_slots(surface, slots, original_image)
    }

    fn inspect(&mut self, inspector: &dyn Inspector) {
        self.upload.inspect(inspector);
    }

    fn input(&mut self, eye: &EyeInput, mouse: &MouseInput) -> EyeInput {
        // Uploading the buffer here is a bit sketchy but works.
        if let Ok(buffer) = self.frame_receiver.try_recv() {
            self.upload.upload_buffer(&buffer);
        }
        self.upload.input(eye, mouse)
    }

    fn render(
        &mut self,
        surface: &Surface,
        encoder: &mut wgpu::CommandEncoder,
        screen: Option<&RenderTexture>,
    ) {
        self.upload.render(surface, encoder, screen);
    }

    fn post_render(&mut self, surface: &Surface) {
        self.upload.post_render(surface);
    }
}

fn build_flow(surface: &mut Surface, frame_receiver: Receiver<RgbBuffer>) {
    //TODO: use a proper set of nodes.

    // Input node.
    let node = UploadStream::new(surface, frame_receiver);
    surface.add_node(Box::new(node), 0);

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
pub struct Simulator {
    frame_sender: SyncSender<RgbBuffer>,
}

#[wasm_bindgen]
impl Simulator {
    #[wasm_bindgen]
    pub fn create_and_run(parent_id: &str) -> Simulator {
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

        let (tx, rx) = mpsc::sync_channel(2);
        wasm_bindgen_futures::spawn_local(window_surface.run_and_exit(
            move |surface| {
                build_flow(surface, rx);
            },
            || false,
        ));

        Simulator { frame_sender: tx }
    }

    #[wasm_bindgen]
    pub fn post_frame(
        &mut self,
        pixels: Vec<u8>,
        width: usize,
        height: usize,
    ) -> Result<(), JsError> {
        let buffer = RgbBuffer {
            pixels_rgb: pixels.into_boxed_slice(),
            width: width as u32,
            height: height as u32,
        };
        self.frame_sender
            .try_send(buffer)
            .map_err(|err| JsError::new(&err.to_string()))
    }
}
