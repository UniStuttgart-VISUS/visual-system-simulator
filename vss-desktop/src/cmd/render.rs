use super::{refresh_flow_configs, report_diagnostics, CommonConfig};
use crate::flow::{build_flow, finalize_flows, FlowError, FlowRequest, FlowStage};
use std::error::Error;
use std::path::{Path, PathBuf};
use vss::*;

struct OutputInfo {
    /// Path without dirname, e.g., `path/to`.
    dirname: String,
    /// Basename without stem, e.g., `png`.
    extension: String,
    /// Basename without extension, e.g., `image`.
    stem: String,
}

#[derive(Debug)]
struct PlannedInput {
    input: String,
    output: PathBuf,
}

#[derive(Debug, clap::Args)]
pub(crate) struct RenderArgs {
    #[arg(short = 'c', long = "config", value_name = "FILE")]
    config: Option<PathBuf>,

    #[arg(
        short = 'o',
        long = "output",
        value_name = "PATTERN",
        default_value = "{dirname}/{stem}.vss.{extension}"
    )]
    output: String,

    #[arg(long = "force", help = "Overwrite existing output files")]
    force: bool,

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
    planned_inputs: Vec<PlannedInput>,
}

pub(crate) fn run(args: RenderArgs) -> Result<(), String> {
    let config = args.into_config().map_err(|err| err.to_string())?;
    run_batch_render(config).map_err(|err| err.to_string())
}

fn run_batch_render(config: RenderConfig) -> Result<(), FlowError> {
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

    let RenderConfig {
        mut common,
        force,
        planned_inputs,
    } = config;
    for planned_input in planned_inputs {
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
        run_headless_render_with_renderer(&common, force, planned_input.output, &renderer)?;
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
) -> Result<(), FlowError> {
    let first_input = config
        .inputs
        .first()
        .cloned()
        .unwrap_or_else(|| "unknown".to_string());
    let mut context = RenderContext::new(
        [1, 1],
        1,
        renderer.device.clone(),
        renderer.queue.clone(),
        wgpu::TextureFormat::Rgba8UnormSrgb,
    );

    let built = build_flow(
        &mut context,
        0,
        FlowRequest {
            input: config.inputs[0].clone(),
            output: Some(output_path),
            force,
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
    let diagnostics = finalize_flows(&mut context, &config.config_document, &[0]);
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
    loop {
        let mut encoder =
            context
                .device()
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("batch render encoder"),
                });
        context.render(&mut encoder, &dummy_screen);
        context.queue().submit(std::iter::once(encoder.finish()));
        context.post_render();

        if built.output_failure.read().unwrap().is_some() || *built.output_processed.read().unwrap()
        {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }

    if let Some(message) = built.output_failure.read().unwrap().clone() {
        return Err(FlowError {
            input: first_input,
            stage: FlowStage::Encode,
            message,
        });
    }

    Ok(())
}

fn input_output_info(path: &Path) -> OutputInfo {
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
    OutputInfo {
        dirname,
        extension,
        stem,
    }
}

fn plan_output_path(pattern: &str, input: &Path) -> Result<PathBuf, Box<dyn Error>> {
    let info = input_output_info(input);
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
            other => return Err(format!("unknown placeholder: {{{other}}}")),
        });

        rest = &rest[end + 1..];
    }

    result.push_str(rest);
    Ok(result)
}

fn expand_input_pattern(input: &str) -> Result<Vec<String>, Box<dyn Error>> {
    if !input.contains(['*', '?', '[']) {
        return Ok(vec![input.to_string()]);
    }

    let mut matches: Vec<PathBuf> = glob::glob(input)?.collect::<Result<_, _>>()?;
    if matches.is_empty() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("unmatched glob pattern: {input}"),
        )
        .into());
    }
    matches.sort();
    Ok(matches
        .into_iter()
        .map(|path| path.to_string_lossy().to_string())
        .collect())
}

fn plan_inputs(
    inputs: &[String],
    output_pattern: &str,
) -> Result<Vec<PlannedInput>, Box<dyn Error>> {
    inputs
        .iter()
        .map(|raw_input| expand_input_pattern(raw_input))
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .flatten()
        .map(|input| {
            let output = plan_output_path(output_pattern, Path::new(&input))?;
            Ok(PlannedInput { input, output })
        })
        .collect()
}

impl RenderArgs {
    fn into_config(self) -> Result<RenderConfig, Box<dyn Error>> {
        let common = CommonConfig {
            inputs: self.input,
            base_config: self.config,
            ..CommonConfig::default()
        };
        let planned_inputs = plan_inputs(&common.inputs, &self.output)?;
        Ok(RenderConfig {
            common,
            force: self.force,
            planned_inputs,
        })
    }
}
