use super::{refresh_flow_configs, report_diagnostics, CommonConfig};
use crate::flow::{build_flow, finalize_flows, FlowRequest};
use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use vss::*;
use vss_winit::*;

pub type OpenXrBackend = vss_openxr::Backend;

#[derive(Debug, Default, clap::Args)]
pub(crate) struct ShowArgs {
    #[arg(short = 'c', long = "config", value_name = "FILE")]
    config: Vec<PathBuf>,

    #[arg(
        long = "openxr",
        value_name = "BACKEND",
        num_args = 0..=1,
        require_equals = true,
        default_missing_value = "auto",
        value_parser = parse_openxr_backend
    )]
    openxr: Option<OpenXrBackend>,

    #[arg(
        value_name = "INPUT",
        help = "Input image or video. Opens a file picker when omitted."
    )]
    input: Option<String>,
}

impl ShowArgs {
    fn into_config(self) -> ShowConfig {
        let mut common = CommonConfig {
            config_paths: self.config,
            ..CommonConfig::default()
        };
        if let Some(input) = self.input {
            common.inputs = vec![input];
        }
        ShowConfig {
            common,
            openxr: self.openxr,
        }
    }
}

#[derive(Debug)]
struct ShowConfig {
    common: CommonConfig,
    openxr: Option<OpenXrBackend>,
}

fn parse_openxr_backend(value: &str) -> Result<OpenXrBackend, String> {
    OpenXrBackend::parse(value).map_err(|err| err.to_string())
}

fn pick_input_file() -> Option<String> {
    let mut dialog = rfd::FileDialog::new();
    if let Ok(current_dir) = std::env::current_dir() {
        dialog = dialog.set_directory(current_dir);
    }
    dialog = dialog.add_filter("Image", &["png", "jpg", "jpeg", "bmp", "gif", "webp"]);
    #[cfg(feature = "video")]
    {
        dialog = dialog.add_filter("Video", &["mp4", "mov", "mkv", "avi", "webm"]);
    }
    dialog
        .pick_file()
        .map(|path| path.to_string_lossy().to_string())
}

pub(crate) fn run(args: ShowArgs) -> Result<(), String> {
    let mut config = args.into_config();
    if config.common.inputs.is_empty() {
        config.common.inputs = vec![pick_input_file().ok_or("no input selected")?];
    }
    let diagnostics = refresh_flow_configs(&mut config.common).map_err(|err| err.to_string())?;
    report_diagnostics(&diagnostics);

    if let Some(backend) = config.openxr.take() {
        return run_openxr(config, backend);
    }

    let mut event_loop = EventLoop::new().map_err(|err| err.to_string())?;
    run_windowed(config, &mut event_loop)
}

fn run_windowed(config: ShowConfig, event_loop: &mut EventLoop<()>) -> Result<(), String> {
    let failure: Arc<RwLock<Option<String>>> = Arc::new(RwLock::new(None));

    let left = config.common.config_document.effective_left();
    let gui = crate::ui::DesktopGui::new(&config.common.config_document);

    let failure_init = failure.clone();
    let pose_input_size: Arc<RwLock<Option<[u32; 2]>>> = Arc::new(RwLock::new(None));
    let pose_input_size_init = pose_input_size.clone();
    let failure_poll = failure.clone();
    let window = WindowSurface::new(
        true,
        1,
        config_point(&left, "view").map(|view| (view[0] as f32, view[1] as f32)),
        config_point(&left, "gaze").map(|gaze| (gaze[0] as f32, gaze[1] as f32)),
        pose_input_size,
        move |surface| {
            let built = match build_flow(
                surface,
                0,
                FlowRequest {
                    input: config.common.inputs[0].clone(),
                    output: None,
                    force: false,
                    render_resolution: RenderResolution::Screen {
                        input_scale: 1.0,
                        output_scale: OutputScale::default(),
                    },
                    view_port: ViewPort {
                        x: 0.0,
                        y: 0.0,
                        width: 1.0,
                        height: 1.0,
                        absolute_viewport: false,
                    },
                },
            ) {
                Ok(built) => built,
                Err(err) => {
                    *failure_init.write().unwrap() = Some(err.to_string());
                    return;
                }
            };
            if let Some(input_size) = built.input_size {
                *pose_input_size_init.write().unwrap() = Some(input_size);
            }
            let diagnostics = finalize_flows(surface, &config.common.config_document, &[0]);
            report_diagnostics(&diagnostics);
        },
        move || failure_poll.read().unwrap().is_some(),
    )
    .with_overlay(gui);

    window.run_app(event_loop).map_err(|err| err.to_string())?;

    if let Some(err) = failure.read().unwrap().clone() {
        return Err(err);
    }

    Ok(())
}

fn run_openxr(config: ShowConfig, backend: OpenXrBackend) -> Result<(), String> {
    let runtime = vss_openxr::Runtime::new(vss_openxr::RuntimeOptions {
        backend,
        loader_path: None,
    });
    let pose_input_size: Arc<RwLock<Option<[u32; 2]>>> = Arc::new(RwLock::new(None));
    let pose_input_size_runtime = pose_input_size.clone();
    runtime
        .run(move |context, views| {
            let eye_indices = views.iter().map(|view| view.eye_index).collect::<Vec<_>>();
            for view in views {
                let effective_section = if view.eye_index == 0 {
                    config.common.config_document.effective_left()
                } else {
                    config.common.config_document.effective_right()
                };
                let viewport = ViewPort {
                    x: view.viewport.x as f32,
                    y: view.viewport.y as f32,
                    width: view.viewport.width as f32,
                    height: view.viewport.height as f32,
                    absolute_viewport: true,
                };
                let built = build_flow(
                    context,
                    view.view_index,
                    FlowRequest {
                        input: config.common.inputs[0].clone(),
                        output: None,
                        force: false,
                        render_resolution: RenderResolution::Buffer { input_scale: 1.0 },
                        view_port: viewport,
                    },
                )
                .map_err(|err| err.to_string())?;
                if let Some(input_size) = built.input_size {
                    let mut pose_input_size = pose_input_size_runtime.write().unwrap();
                    pose_input_size.get_or_insert(input_size);
                }

                let pose_size = pose_input_size_runtime
                    .read()
                    .unwrap()
                    .unwrap_or([view.viewport.width.max(1), view.viewport.height.max(1)]);

                let mut eye = context.flows[view.view_index].eye_mut();
                if let Some([x, y]) = config_point(&effective_section, "view") {
                    eye.view = pose_from_position((x as f32, y as f32), pose_size).0;
                }
                if let Some([x, y]) = config_point(&effective_section, "gaze") {
                    eye.gaze = pose_from_position((x as f32, y as f32), pose_size).1;
                }
            }
            let diagnostics = finalize_flows(context, &config.common.config_document, &eye_indices);
            report_diagnostics(&diagnostics);
            Ok(())
        })
        .map_err(|err| err.to_string())
}

fn config_point(section: &vss_catalog::EffectiveSection, id: &str) -> Option<[f64; 2]> {
    let values = section.values.get(id)?.value.as_array()?;
    (values.len() == 2).then_some([values[0].as_f64()?, values[1].as_f64()?])
}
