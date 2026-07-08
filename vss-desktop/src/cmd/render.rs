use super::{refresh_flow_configs, report_diagnostics, CommonConfig};
use crate::flow::{build_flow, finalize_flows, FlowError, FlowRequest, FlowStage};
use std::collections::HashSet;
use std::error::Error;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use vss::*;

struct OutputInfo {
    /// Path without dirname, e.g., `path/to`.
    dirname: String,
    /// Basename without stem, e.g., `png`.
    extension: String,
    /// Basename without extension, e.g., `image`.
    stem: String,
    /// Configuration basename without extension, e.g., `normal`.
    config_stem: String,
}

#[derive(Debug)]
struct PlannedInput {
    config: Option<PathBuf>,
    input: String,
    output: PathBuf,
}

struct PendingImageOutput {
    input: String,
    completion: std::sync::mpsc::Receiver<Result<(), String>>,
}

impl PendingImageOutput {
    fn wait(self) -> Result<(), FlowError> {
        let result = self.completion.recv().map_err(|err| FlowError {
            input: self.input.clone(),
            stage: FlowStage::Encode,
            message: format!("image encoder stopped before reporting completion: {err}"),
        })?;
        result.map_err(|message| FlowError {
            input: self.input,
            stage: FlowStage::Encode,
            message,
        })
    }
}

#[derive(Debug, clap::Args)]
pub(crate) struct RenderArgs {
    #[arg(
        short = 'c',
        long = "config",
        value_name = "FILE_OR_PATTERN",
        help = "Configuration files and glob patterns; may be repeated"
    )]
    config: Vec<String>,

    #[arg(
        short = 'o',
        long = "output",
        value_name = "PATTERN",
        default_value = "{dirname}/{stem}.{config}.{extension}",
        help = "Output pattern ({dirname}, {stem}, {extension}, {config})"
    )]
    output: String,

    #[arg(long = "force", help = "Overwrite existing output files")]
    force: bool,

    #[arg(
        short = 'v',
        long = "verbose",
        help = "Print timing information for each render stage"
    )]
    verbose: bool,

    #[arg(
        value_name = "INPUT",
        required = true,
        num_args = 1..,
        help = "Input images, videos, and glob patterns"
    )]
    input: Vec<String>,
}

#[derive(Debug)]
struct RenderConfig {
    common: CommonConfig,
    force: bool,
    verbose: bool,
    planned_inputs: Vec<PlannedInput>,
}

fn format_duration(duration: Duration) -> String {
    format!("{:.3} ms", duration.as_secs_f64() * 1000.0)
}

pub(crate) fn run(args: RenderArgs) -> Result<(), String> {
    let config = args.into_config().map_err(|err| err.to_string())?;
    run_batch_render(config).map_err(|err| err.to_string())
}

fn run_batch_render(config: RenderConfig) -> Result<(), FlowError> {
    let batch_start = Instant::now();
    let renderer_start = Instant::now();
    let renderer = create_headless_renderer().map_err(|message| FlowError {
        input: config
            .common
            .inputs
            .first()
            .cloned()
            .unwrap_or_else(|| "unknown".to_string()),
        stage: FlowStage::Encode,
        message,
    })?;
    let renderer_time = renderer_start.elapsed();

    let RenderConfig {
        mut common,
        force,
        verbose,
        planned_inputs,
    } = config;
    if verbose {
        eprintln!("render: device setup {}", format_duration(renderer_time));
    }
    let mut pending_outputs = Vec::new();
    for planned_input in planned_inputs {
        if !force && planned_input.output.exists() {
            eprintln!(
                "encode error for {}: output file already exists: {}; pass --force to overwrite",
                planned_input.input,
                planned_input.output.display()
            );
            continue;
        }
        let config_start = Instant::now();
        common.base_config = planned_input.config.clone();
        common.inputs = vec![planned_input.input.clone()];
        let diagnostics = refresh_flow_configs(&mut common).map_err(|err| FlowError {
            input: planned_input.input.clone(),
            stage: FlowStage::Decode,
            message: err.to_string(),
        })?;
        if report_diagnostics(&diagnostics).is_err() {
            return Err(FlowError {
                input: planned_input.input,
                stage: FlowStage::Decode,
                message: "configuration validation failed".to_string(),
            });
        }
        let config_time = config_start.elapsed();
        if let Some(pending) = run_headless_render_with_renderer(
            &common,
            force,
            planned_input.output,
            &renderer,
            verbose,
            config_time,
        )? {
            pending_outputs.push(pending);
        }
    }

    let mut first_error = None;
    for pending in pending_outputs {
        if let Err(err) = pending.wait() {
            first_error.get_or_insert(err);
        }
    }
    if let Some(err) = first_error {
        return Err(err);
    }
    if verbose {
        eprintln!(
            "render: batch total {}",
            format_duration(batch_start.elapsed())
        );
    }

    Ok(())
}

struct HeadlessRenderer {
    device: wgpu::Device,
    queue: wgpu::Queue,
}

fn create_headless_renderer() -> Result<HeadlessRenderer, String> {
    let instance = wgpu::Instance::default();
    let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::HighPerformance,
        compatible_surface: None,
        force_fallback_adapter: false,
    }))
    .map_err(|err| format!("Unable to find a rendering adapter: {err}"))?;
    let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
        label: Some("VSS batch render device"),
        required_features: wgpu::Features::empty(),
        required_limits: wgpu::Limits::default(),
        experimental_features: wgpu::ExperimentalFeatures::disabled(),
        memory_hints: wgpu::MemoryHints::Performance,
        trace: wgpu::Trace::Off,
    }))
    .map_err(|err| format!("Unable to create a rendering device: {err}"))?;

    Ok(HeadlessRenderer { device, queue })
}

fn run_headless_render_with_renderer(
    config: &CommonConfig,
    force: bool,
    output_path: PathBuf,
    renderer: &HeadlessRenderer,
    verbose: bool,
    config_time: Duration,
) -> Result<Option<PendingImageOutput>, FlowError> {
    let first_input = config
        .inputs
        .first()
        .cloned()
        .unwrap_or_else(|| "unknown".to_string());
    let context_start = Instant::now();
    let mut context = RenderContext::new(
        [1, 1],
        1,
        renderer.device.clone(),
        renderer.queue.clone(),
        wgpu::TextureFormat::Rgba8UnormSrgb,
    );
    let context_time = context_start.elapsed();

    let built = build_flow(
        &mut context,
        0,
        FlowRequest {
            input: config.inputs[0].clone(),
            output: Some(output_path),
            force,
            show_gui: false,
            render_resolution: RenderResolution::Buffer { input_scale: 1.0 },
            view_port: ViewPort {
                x: 0.0,
                y: 0.0,
                width: 1.0,
                height: 1.0,
                absolute_viewport: false,
            },
        },
    )?;
    let finalize_start = Instant::now();
    let diagnostics = finalize_flows(&mut context, &config.config_document, &[0]);
    let finalize_time = finalize_start.elapsed();
    if report_diagnostics(&diagnostics).is_err() {
        return Err(FlowError {
            input: first_input,
            stage: FlowStage::Encode,
            message: "configuration validation failed".to_string(),
        });
    }

    let dummy_screen = RenderTexture::empty_color_with_format(
        context.device(),
        context.output_format(),
        Some("batch render dummy screen"),
    );
    let render_start = Instant::now();
    render_headless_frame(&mut context, &dummy_screen);
    let render_time = render_start.elapsed();

    if verbose {
        eprintln!(
            "render {}: config {}, context {}, endpoints/decode {}, graph {}, finalize {}, gpu/readback {}",
            first_input,
            format_duration(config_time),
            format_duration(context_time),
            format_duration(built.endpoint_setup_time),
            format_duration(built.graph_build_time),
            format_duration(finalize_time),
            format_duration(render_time),
        );
    }

    if built.render_once {
        return Ok(Some(PendingImageOutput {
            input: first_input,
            completion: built
                .output_completion
                .expect("image output must have a completion receiver"),
        }));
    }

    while built.output_failure.read().unwrap().is_none() && !*built.output_processed.read().unwrap()
    {
        render_headless_frame(&mut context, &dummy_screen);
    }

    if let Some(message) = built.output_failure.read().unwrap().clone() {
        return Err(FlowError {
            input: first_input,
            stage: FlowStage::Encode,
            message,
        });
    }

    Ok(None)
}

fn render_headless_frame(context: &mut RenderContext, screen: &RenderTexture) {
    let mut changes = NodeChanges::empty();
    for flow in &context.flows {
        changes |= flow.input(&MouseInput::default());
    }
    context.apply_changes(changes);
    if changes.contains(NodeChanges::SLOTS) {
        context.negociate_slots();
    }

    let mut encoder = context
        .device()
        .create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("batch render encoder"),
        });
    context.render(&mut encoder, screen);
    context.queue().submit(std::iter::once(encoder.finish()));
    context.post_render();
}

fn input_output_info(path: &Path, config: Option<&Path>) -> OutputInfo {
    let dirname = match path.parent() {
        Some(parent) if !parent.as_os_str().is_empty() => parent.to_string_lossy().to_string(),
        _ => ".".to_string(),
    };
    let stem = path
        .file_stem()
        .map(|stem| stem.to_string_lossy().to_string())
        .unwrap_or_default();
    let extension = path
        .extension()
        .map(|extension| extension.to_string_lossy().to_string())
        .unwrap_or_default();
    let config_stem = config
        .and_then(Path::file_stem)
        .map(|stem| stem.to_string_lossy().to_string())
        .unwrap_or_else(|| "vss".to_string());
    OutputInfo {
        dirname,
        extension,
        stem,
        config_stem,
    }
}

fn plan_output_path(
    pattern: &str,
    input: &Path,
    config: Option<&Path>,
) -> Result<PathBuf, Box<dyn Error>> {
    let info = input_output_info(input, config);
    let output = format_output_pattern(pattern, &info)
        .map_err(|err| std::io::Error::new(std::io::ErrorKind::InvalidInput, err))?;
    Ok(PathBuf::from(output))
}

fn format_output_pattern(pattern: &str, info: &OutputInfo) -> Result<String, String> {
    let mut result = String::new();
    let mut rest = pattern;

    while let Some(start) = rest.find('{') {
        result.push_str(&rest[..start]);
        rest = &rest[start + 1..];

        let end = rest.find('}').ok_or("unterminated placeholder")?;
        let placeholder = &rest[..end];

        result.push_str(match placeholder {
            "dirname" => &info.dirname,
            "stem" => &info.stem,
            "extension" => &info.extension,
            "config" => &info.config_stem,
            other => return Err(format!("unknown placeholder: {{{other}}}")),
        });

        rest = &rest[end + 1..];
    }

    result.push_str(rest);
    Ok(result)
}

fn expand_path_pattern(pattern: &str) -> Result<Vec<PathBuf>, Box<dyn Error>> {
    if !pattern.contains(['*', '?', '[']) {
        return Ok(vec![PathBuf::from(pattern)]);
    }

    let mut matches: Vec<PathBuf> = glob::glob(pattern)?.collect::<Result<_, _>>()?;
    if matches.is_empty() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("unmatched glob pattern: {pattern}"),
        )
        .into());
    }
    matches.sort();
    Ok(matches)
}

fn expand_patterns(patterns: &[String]) -> Result<Vec<PathBuf>, Box<dyn Error>> {
    patterns
        .iter()
        .map(|pattern| expand_path_pattern(pattern))
        .collect::<Result<Vec<_>, _>>()
        .map(|matches| matches.into_iter().flatten().collect())
}

fn plan_inputs(
    inputs: &[String],
    configs: &[String],
    output_pattern: &str,
) -> Result<Vec<PlannedInput>, Box<dyn Error>> {
    let inputs = expand_patterns(inputs)?;
    let configs = if configs.is_empty() {
        vec![None]
    } else {
        expand_patterns(configs)?.into_iter().map(Some).collect()
    };
    let mut outputs = HashSet::new();
    let mut planned = Vec::new();

    for config in configs {
        for input in &inputs {
            let output = plan_output_path(output_pattern, input, config.as_deref())?;
            if !outputs.insert(output.clone()) {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::AlreadyExists,
                    format!(
                        "multiple renders would write {}; include {{config}} and/or {{stem}} in --output",
                        output.display()
                    ),
                )
                .into());
            }
            planned.push(PlannedInput {
                config: config.clone(),
                input: input.to_string_lossy().to_string(),
                output,
            });
        }
    }

    Ok(planned)
}

impl RenderArgs {
    fn into_config(self) -> Result<RenderConfig, Box<dyn Error>> {
        let common = CommonConfig {
            inputs: self.input,
            ..CommonConfig::default()
        };
        let planned_inputs = plan_inputs(&common.inputs, &self.config, &self.output)?;
        Ok(RenderConfig {
            common,
            force: self.force,
            verbose: self.verbose,
            planned_inputs,
        })
    }
}
