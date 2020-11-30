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
        self.input_idx += 1;
        self.current(&window)
    }

    fn current(&mut self, window: &Window) -> Option<IoNodePair> {
        if self.input_idx >= self.inputs.len() {
            None
        } else {
            let input = &self.inputs[self.input_idx];
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

fn build_flow(window: &mut Window, io_generator: &mut IoGenerator, flow_index: usize){
    let (input_node, output_node) = io_generator.current(&window).unwrap();

    // Add input node.
    window.add_node(input_node, flow_index);

    #[cfg(feature = "varjo")]{
        let node = Seperator::new(&window);
        window.add_node(Box::new(node), flow_index);
    }

    // Visual system passes.
    let node = Cataract::new(&window);
    window.add_node(Box::new(node), flow_index);
    let node = Lens::new(&window);
    window.add_node(Box::new(node), flow_index);
    let node = Retina::new(&window);
    window.add_node(Box::new(node), flow_index);

    // Add output node, if present.
    if let Some(output_node) = output_node {
        window.add_node(output_node, flow_index);
    } else {
        window.add_node(Box::new(Passthrough::new(&window)), flow_index)
    }

    // Display node.
    let node = Display::new(&window);
    window.add_node(Box::new(node), flow_index);

    #[cfg(feature = "varjo")]{
        let node = Compositor::new(&window);
        window.add_node(Box::new(node), flow_index);
        window.update_nodes();
    }
}

pub fn main() {
    let config = cmd_parse();

    let remote = if let Some(port) = config.port {
        Some(Remote::new(port))
    } else {
        None
    };
    
    #[cfg(not(feature = "varjo"))]
    let flow_count = 1;
    #[cfg(feature = "varjo")]
    let flow_count = 4;

    let mut window = Window::new(config.visible, remote, config.parameters, flow_count);

    #[cfg(feature = "varjo")]
    let mut varjo = varjo::Varjo::new();
    #[cfg(feature = "varjo")]//TODO: used to reduce log spam, remove when no longer needed or replace with a better solution
    let mut log_counter = 0;
    #[cfg(feature = "varjo")]
    varjo.create_render_targets(&window);

    let mut io_generator = IoGenerator::new(config.inputs, config.output);
    
    for index in 0 .. flow_count {
        build_flow(&mut window, &mut io_generator, index);
    }

    let mut done = false;
    while !done {
        #[cfg(feature = "varjo")]{
            varjo.logging_enabled = log_counter == 0;
            if !varjo.begin_frame_sync() {continue;}
            let (varjo_target_color, varjo_target_depth) = varjo.get_current_render_target();
            window.replace_targets(varjo_target_color, varjo_target_depth, false);
            window.set_head(varjo.get_current_view_matrices(), varjo.get_current_proj_matrices());
            varjo.get_current_gaze();
            
            window.update_last_node();
        }
        
        done = window.poll_events();

        #[cfg(feature = "varjo")]{
            varjo.end_frame();
            log_counter = (log_counter+1) % 10;
        }

        if io_generator.is_ready() {
            if let Some((input_node, output_node)) = io_generator.next(&window) {
                window.replace_node(0, input_node, 0);
                let output_node = if let Some(output_node) = output_node {
                    output_node
                } else {
                    Box::new(Passthrough::new(&window))
                };
                window.replace_node(window.nodes_len() - 2, output_node, 0);
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
