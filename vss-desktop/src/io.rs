use crate::{node::*, OutputInfo};
use vss::*;

pub type IoNodePair = (Box<dyn Node>, Option<Box<dyn Node>>);

pub struct IoGenerator {
    pub(crate) inputs: Vec<String>,
    pub(crate) config_name: String,
    pub(crate) output: Option<mustache::Template>,
    pub(crate) input_idx: usize,
    pub(crate) input_processed: std::sync::Arc<std::sync::RwLock<bool>>,
}

impl IoGenerator {
    pub fn new(
        inputs: Vec<String>,
        config_name: String,
        output: Option<mustache::Template>,
    ) -> Self {
        Self {
            inputs,
            config_name,
            output,
            input_idx: 0,
            input_processed: std::sync::Arc::new(std::sync::RwLock::new(false)),
        }
    }

    pub fn _is_ready(&self) -> bool {
        *self.input_processed.read().unwrap()
    }

    pub fn _next(
        &mut self,
        context: &RenderContext,
        render_resolution: Option<[u32; 2]>,
    ) -> Option<IoNodePair> {
        self.input_idx += 1;
        let render_res = if let Some(res) = render_resolution {
            RenderResolution::Custom { res }
        } else {
            RenderResolution::Buffer { input_scale: 1.0 } //TODO add input scaling
        };
        self.current(context, render_res, 0)
    }

    pub fn current(
        &mut self,
        context: &RenderContext,
        render_resolution: RenderResolution,
        flow_index: usize,
    ) -> Option<IoNodePair> {
        if self.input_idx >= self.inputs.len() {
            None
        } else {
            let input = &self.inputs[self.input_idx];
            if UploadRgbBuffer::has_image_extension(input) {
                let input_path = std::path::Path::new(input);
                let mut input_node = UploadRgbBuffer::new(context);
                input_node.upload_image(load(input_path));
                input_node.set_flags(
                    RgbInputFlags::from_extension(input) | RgbInputFlags::VERTICALLY_FLIPPED,
                );
                input_node.set_render_resolution(render_resolution);
                let output_node = if let Some(output) = &self.output {
                    let mut output_node = DownloadRgbBuffer::new(context);
                    let output_info = OutputInfo {
                        configname: self.config_name.clone(),
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
                            .unwrap()
                            + &format!("_{}", flow_index),
                        extension: input_path
                            .extension()
                            .unwrap()
                            .to_os_string()
                            .into_string()
                            .unwrap(),
                    };
                    let output_path = output.render_to_string(&output_info).unwrap();
                    output_node.set_image_path(output_path, self.input_processed.clone());
                    Some(Box::new(output_node) as Box<dyn Node>)
                } else {
                    None
                };
                Some((Box::new(input_node), output_node))
            } else if UploadVideo::has_video_extension(input) {
                let mut input_node = UploadVideo::new(context);
                input_node.set_flags(RgbInputFlags::from_extension(input));
                input_node.open(input).unwrap();
                Some((Box::new(input_node), None))
            } else {
                panic!("Unknown file extension");
            }
        }
    }
}
