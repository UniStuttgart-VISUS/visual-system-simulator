use crate::node::*;
use std::collections::BTreeMap;
use std::fmt;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};
use tempfile::NamedTempFile;
use vss::*;

type EndpointNodes = (Box<dyn Node>, Option<Box<dyn Node>>);

#[derive(Debug)]
pub(crate) enum FlowStage {
    Decode,
    Encode,
}

impl fmt::Display for FlowStage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FlowStage::Decode => f.write_str("decode"),
            FlowStage::Encode => f.write_str("encode"),
        }
    }
}

#[derive(Debug)]
pub(crate) struct FlowError {
    pub(crate) input: String,
    pub(crate) stage: FlowStage,
    pub(crate) message: String,
}

impl fmt::Display for FlowError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} error for {}: {}",
            self.stage, self.input, self.message
        )
    }
}

impl std::error::Error for FlowError {}

struct Endpoints {
    nodes: EndpointNodes,
    input_size: Option<[u32; 2]>,
    render_once: bool,
    output_completion: Option<std::sync::mpsc::Receiver<Result<(), String>>>,
    output_processed: Arc<RwLock<bool>>,
    output_failure: Arc<RwLock<Option<String>>>,
}

fn create_endpoints(
    context: &RenderContext,
    input: &str,
    output: Option<PathBuf>,
    force: bool,
    render_resolution: RenderResolution,
) -> Result<Endpoints, FlowError> {
    let input_processed = Arc::new(RwLock::new(false));
    let output_failure = Arc::new(RwLock::new(None));
    if UploadRgbBuffer::has_image_extension(input) {
        let input_path = Path::new(input);
        let mut input_node = UploadRgbBuffer::new(context);
        input_node
            .upload_image(load_input_bytes(input_path).map_err(|message| FlowError {
                input: input.to_string(),
                stage: FlowStage::Decode,
                message,
            })?)
            .map_err(|message| FlowError {
                input: input.to_string(),
                stage: FlowStage::Decode,
                message,
            })?;
        input_node
            .set_flags(RgbInputFlags::from_extension(input) | RgbInputFlags::VERTICALLY_FLIPPED);
        input_node.set_render_resolution(render_resolution);
        let input_size = input_node.input_size();
        let (output_node, output_completion) = if let Some(output_path) = output {
            let mut output_node = DownloadRgbBuffer::new(context);
            let processed = input_processed.clone();
            let failure = output_failure.clone();
            let (completion_tx, completion_rx) = std::sync::mpsc::channel();
            let format = ImageFormat::from_path(&output_path).map_err(|err| FlowError {
                input: input.to_string(),
                stage: FlowStage::Encode,
                message: format!(
                    "failed to detect image format for {}: {err}",
                    output_path.display()
                ),
            })?;
            output_node.set_image_encoder(
                format,
                move |encoded| save_bytes_atomically(&output_path, &encoded, force),
                processed,
                failure,
                completion_tx,
            );
            (
                Some(Box::new(output_node) as Box<dyn Node>),
                Some(completion_rx),
            )
        } else {
            (None, None)
        };
        Ok(Endpoints {
            nodes: (Box::new(input_node), output_node),
            input_size,
            render_once: true,
            output_completion,
            output_processed: input_processed,
            output_failure,
        })
    } else if UploadVideo::has_video_extension(input) {
        let mut input_node = UploadVideo::new(context);
        input_node.set_flags(RgbInputFlags::from_extension(input));
        input_node.open(input).map_err(|message| FlowError {
            input: input.to_string(),
            stage: FlowStage::Decode,
            message: message.to_string(),
        })?;
        #[cfg(feature = "video")]
        let batch_state = if output.is_some() {
            Some(
                input_node
                    .enable_batch_render()
                    .map_err(|message| FlowError {
                        input: input.to_string(),
                        stage: FlowStage::Decode,
                        message: message.to_string(),
                    })?,
            )
        } else {
            None
        };
        let input_size = input_node.input_size();
        #[cfg(feature = "video")]
        let output_node = if let (Some(output_path), Some(batch_state)) = (output, batch_state) {
            Some(Box::new(DownloadVideo::new(
                context,
                output_path.clone(),
                force,
                batch_state,
                input_processed.clone(),
                output_failure.clone(),
            )) as Box<dyn Node>)
        } else {
            None
        };
        #[cfg(not(feature = "video"))]
        let output_node = None;
        Ok(Endpoints {
            nodes: (Box::new(input_node), output_node),
            input_size,
            render_once: false,
            output_completion: None,
            output_processed: input_processed,
            output_failure,
        })
    } else {
        Err(FlowError {
            input: input.to_string(),
            stage: FlowStage::Decode,
            message: "unknown input file extension".to_string(),
        })
    }
}

fn save_bytes_atomically(path: &Path, bytes: &[u8], force: bool) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent).map_err(|err| {
                format!(
                    "failed to create output directory {}: {err}",
                    parent.display()
                )
            })?;
        }
    }

    let mut temp_file = NamedTempFile::new_in(path.parent().unwrap_or_else(|| Path::new(".")))
        .map_err(|err| {
            format!(
                "failed to create temporary output for {}: {err}",
                path.display()
            )
        })?;
    let temp_path = temp_file.path().to_path_buf();
    {
        let temp_writer = temp_file.as_file_mut();
        temp_writer.write_all(bytes).map_err(|err| {
            format!(
                "failed to write temporary output {}: {err}",
                temp_path.display()
            )
        })?;
        temp_writer.flush().map_err(|err| {
            format!(
                "failed to flush temporary output {}: {err}",
                temp_path.display()
            )
        })?;
    }
    let persist_result = if force {
        temp_file.persist(path)
    } else {
        temp_file.persist_noclobber(path)
    };
    if let Err(err) = persist_result {
        return Err(format!(
            "failed to move temporary output into place {}: {err}",
            path.display()
        ));
    }
    Ok(())
}

fn load_input_bytes(path: &Path) -> Result<std::io::Cursor<Vec<u8>>, String> {
    std::fs::read(path)
        .map(std::io::Cursor::new)
        .map_err(|err| format!("failed to read input file {}: {err}", path.display()))
}

pub(crate) struct FlowRequest {
    pub(crate) input: String,
    pub(crate) output: Option<PathBuf>,
    pub(crate) force: bool,
    pub(crate) show_gui: bool,
    pub(crate) render_resolution: RenderResolution,
    pub(crate) view_port: ViewPort,
}

pub(crate) struct BuiltFlow {
    pub(crate) input_size: Option<[u32; 2]>,
    pub(crate) render_once: bool,
    pub(crate) endpoint_setup_time: Duration,
    pub(crate) graph_build_time: Duration,
    pub(crate) output_completion: Option<std::sync::mpsc::Receiver<Result<(), String>>>,
    pub(crate) output_processed: Arc<RwLock<bool>>,
    pub(crate) output_failure: Arc<RwLock<Option<String>>>,
}

pub(crate) fn build_flow(
    context: &mut RenderContext,
    flow_index: usize,
    request: FlowRequest,
) -> Result<BuiltFlow, FlowError> {
    let endpoint_setup_start = Instant::now();
    let endpoints = create_endpoints(
        context,
        &request.input,
        request.output,
        request.force,
        request.render_resolution,
    )?;
    let endpoint_setup_time = endpoint_setup_start.elapsed();
    let Endpoints {
        nodes: (input_node, output_node),
        input_size,
        render_once,
        output_completion,
        output_processed,
        output_failure,
    } = endpoints;

    let graph_build_start = Instant::now();
    context.add_node(input_node, flow_index);
    //TODO: when using OpenXR: context.add_node(Box::new(EyeControl::new(context)), flow_index);
    context.add_node(Box::new(Cataract::new(context)), flow_index);
    context.add_node(Box::new(Lens::new(context)), flow_index);
    context.add_node(Box::new(Retina::new(context)), flow_index);
    context.add_node(Box::new(PeacockCB::new(context)), flow_index);
    context.add_node(Box::new(VarianceMeasure::new(context)), flow_index);
    context.add_node(Box::new(VisOverlay::new(context)), flow_index);

    let mut display = Display::new(context);
    display.set_viewport(request.view_port);
    display.set_output_scale(OutputScale::default());
    context.add_node(Box::new(display), flow_index);
    if request.show_gui {
        context.add_node(Box::new(GuiOverlay::new(context)), flow_index);
    }

    if let Some(output_node) = output_node {
        context.add_node(output_node, flow_index);
    }
    let graph_build_time = graph_build_start.elapsed();

    Ok(BuiltFlow {
        input_size,
        render_once,
        endpoint_setup_time,
        graph_build_time,
        output_completion,
        output_processed,
        output_failure,
    })
}

fn validate_and_apply_simulator(
    flow: &Flow,
    simulator: &BTreeMap<String, ConfigValue>,
    apply: bool,
) -> (NodeChanges, Vec<Diagnostic>) {
    let schema = schema_from_flow(flow);
    let mut diagnostics = validate_simulator_values(simulator, &schema);
    let mut result = NodeChanges::empty();
    if apply && !diagnostics_have_errors(&diagnostics) {
        let (apply_result, apply_diagnostics) =
            apply_simulator_values(flow, simulator, &resolve_asset_reference);
        result |= apply_result;
        diagnostics.extend(apply_diagnostics);
    }
    (result, diagnostics)
}

pub(crate) fn finalize_flows(
    context: &mut RenderContext,
    config_document: &ConfigDocument,
    eye_indices: &[usize],
) -> Vec<Diagnostic> {
    assert_eq!(context.flows.len(), eye_indices.len());
    assert!(eye_indices.iter().all(|eye_index| *eye_index < 2));
    context.negociate_slots();

    let mut diagnostics = Vec::new();
    let mut configure_result = NodeChanges::empty();
    let mut configured_eyes = [false; 2];
    for (flow_index, flow) in context.flows.iter().enumerate() {
        let eye_index = eye_indices[flow_index];
        let section = if eye_index == 0 {
            config_document.effective_left()
        } else {
            config_document.effective_right()
        };
        let (flow_result, flow_diagnostics) =
            validate_and_apply_simulator(flow, &section.simulator_value_map(), true);
        configure_result |= flow_result;
        diagnostics.extend(flow_diagnostics);
        configured_eyes[eye_index] = true;
    }

    if !configured_eyes[1] {
        if let Some(flow) = context.flows.first() {
            let (_, flow_diagnostics) = validate_and_apply_simulator(
                flow,
                &config_document.effective_right().simulator_value_map(),
                false,
            );
            diagnostics.extend(flow_diagnostics);
        }
    }

    if configure_result.contains(NodeChanges::SLOTS) {
        context.negociate_slots();
    }
    context.apply_changes(configure_result);

    diagnostics
}

fn resolve_asset_reference(source: &str, reference: &str) -> AssetId {
    let reference_path = Path::new(reference);
    if reference_path.is_absolute() {
        return AssetId::from_str(reference);
    }

    let base_dir = Path::new(source).parent().and_then(|parent| {
        if parent.as_os_str().is_empty() {
            None
        } else {
            Some(parent)
        }
    });
    let resolved = base_dir
        .map(|base| base.join(reference_path))
        .unwrap_or_else(|| PathBuf::from(reference));
    AssetId::from_str(resolved.to_string_lossy().to_string())
}
