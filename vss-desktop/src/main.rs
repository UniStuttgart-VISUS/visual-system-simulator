mod cmd;
mod node;
#[cfg(feature = "varjo")]
mod varjo;

use vss::*;

use crate::cmd::*;
use crate::node::*;

type IoNodePair = (Box<dyn Node>, Option<Box<dyn Node>>);

struct IoGenerator {
    inputs: Vec<String>,
    output: Option<mustache::Template>,
    input_idx: usize,
    input_processed: std::sync::Arc<std::sync::RwLock<bool>>,
}

impl IoGenerator {
    fn new(inputs: Vec<String>, output: Option<mustache::Template>) -> Self {
        Self {
            inputs,
            output,
            input_idx: 0,
            input_processed: std::sync::Arc::new(std::sync::RwLock::new(false)),
        }
    }

    fn is_ready(&self) -> bool {
        *self.input_processed.read().unwrap()
    }

    fn next(&mut self, window: &Window) -> Option<IoNodePair> {
        if self.input_idx >= self.inputs.len() {
            None
        } else {
            let input = &self.inputs[self.input_idx];
            self.input_idx += 1;
            if UploadRgbBuffer::has_image_extension(&input) {
                let input_path = std::path::Path::new(input);
                let mut input_node = UploadRgbBuffer::new(&window);
                input_node.upload_image(load(input_path));
                input_node.set_flags(RgbInputFlags::from_extension(&input));
                let output_node = if let Some(output) = &self.output {
                    let mut output_node = DownloadRgbBuffer::new(&window);
                    let input_info = InputInfo {
                        dirname: input_path
                            .parent()
                            .unwrap()
                            .to_path_buf()
                            .into_os_string()
                            .into_string()
                            .unwrap(),
                        basename: input_path
                            .file_name()
                            .unwrap()
                            .to_os_string()
                            .into_string()
                            .unwrap(),
                        stem: input_path
                            .file_stem()
                            .unwrap()
                            .to_os_string()
                            .into_string()
                            .unwrap(),
                        extension: input_path
                            .extension()
                            .unwrap()
                            .to_os_string()
                            .into_string()
                            .unwrap(),
                    };
                    let output_path = output.render_to_string(&input_info).unwrap();
                    output_node.set_image_path(output_path, self.input_processed.clone());
                    Some(Box::new(output_node) as Box<dyn Node>)
                } else {
                    None
                };
                Some((Box::new(input_node), output_node))
            } else if UploadVideo::has_video_extension(&input) {
                let mut input_node = UploadVideo::new(&window);
                input_node.set_flags(RgbInputFlags::from_extension(&input));
                input_node.open(input).unwrap();
                Some((Box::new(input_node), None))
            } else {
                panic!("Unknown file extension");
            }
        }
    }
}

pub fn main() {
    let config = cmd_parse();

    let remote = if let Some(port) = config.port {
        Some(Remote::new(port))
    } else {
        None
    };
    let mut window = Window::new(config.visible, remote, config.parameters);

    #[cfg(feature = "varjo")]
    let varjo = varjo::Varjo::new();

    let mut io_generator = IoGenerator::new(config.inputs, config.output);
    let (input_node, output_node) = io_generator.next(&window).unwrap();

    // Add input node.
    window.add_node(input_node);

    // Visual system passes.
    let node = Cataract::new(&window);
    window.add_node(Box::new(node));
    let node = Lens::new(&window);
    window.add_node(Box::new(node));
    let node = Retina::new(&window);
    window.add_node(Box::new(node));

    // Add output node, if present.
    if let Some(output_node) = output_node {
        window.add_node(output_node);
    } else {
        window.add_node(Box::new(Passthrough::new(&window)))
    }

    // Display node.
    let node = Display::new(&window);
    window.add_node(Box::new(node));

    let mut done = false;
    while !done {
        #[cfg(feature = "varjo")]
        varjo.begin_frame_sync();

        done = window.poll_events();

        #[cfg(feature = "varjo")]
        varjo.end_frame();

        if io_generator.is_ready() {
            if let Some((input_node, output_node)) = io_generator.next(&window) {
                window.replace_node(0, input_node);
                let output_node = if let Some(output_node) = output_node {
                    output_node
                } else {
                    Box::new(Passthrough::new(&window))
                };
                window.replace_node(window.nodes_len() - 2, output_node);
                window.update_nodes();
            } else {
                if !config.visible {
                    // Exit once all inputs have been processed, unless visible.
                    done = true;
                }
            }
        }
    }
}
