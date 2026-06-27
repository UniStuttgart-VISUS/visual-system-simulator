mod cmd;
mod io;
mod node;

use std::io::Cursor;
use std::sync::{Arc, Mutex, RwLock};
use std::time::Instant;
use vss::*;
use vss_winit::*;

use crate::cmd::*;
use crate::io::*;

fn build_flow(
    context: &mut RenderContext,
    io_generator: &mut IoGenerator,
    flow_index: usize,
    render_resolution: Option<(u32, u32)>,
    view_port: ViewPort,
    output_scale: OutputScale,
) {
    let render_res = if let Some(res) = render_resolution {
        RenderResolution::Custom {
            res: [res.0, res.1],
        }
    } else {
        RenderResolution::Screen {
            input_scale: 1.0, //TODO add input scaling
            output_scale,
        }
    };
    let (input_node, output_node) = io_generator
        .current(context, render_res, flow_index)
        .unwrap();

    // Add input node.
    context.add_node(input_node, flow_index);

    // Visual system passes.
    let node = Cataract::new(context);
    context.add_node(Box::new(node), flow_index);
    let node = Lens::new(context);
    context.add_node(Box::new(node), flow_index);
    let node = Retina::new(context);
    context.add_node(Box::new(node), flow_index);
    let node = PeacockCB::new(context);
    context.add_node(Box::new(node), flow_index);

    // Measurement Nodes for variance and error.
    let node = VarianceMeasure::new(context);
    context.add_node(Box::new(node), flow_index);
    let node = VisOverlay::new(context);
    context.add_node(Box::new(node), flow_index);

    // Display node.
    let mut node = Display::new(context);
    node.set_viewport(view_port);
    node.set_output_scale(output_scale);
    context.add_node(Box::new(node), flow_index);

    // Add UI overlay.
    let node = GuiOverlay::new(context);
    context.add_node(Box::new(node), flow_index);

    // Add output node, if present.
    if let Some(output_node) = output_node {
        context.add_node(output_node, flow_index);
    }

    context.negociate_slots();
}

pub fn load_fn(full_path: &str) -> Cursor<Vec<u8>> {
    use std::fs::File;
    use std::io::Read;

    let mut file = match File::open(full_path) {
        Ok(file) => file,
        Err(err) => {
            panic!("Cannot open file '{}' ({})", full_path, err);
        }
    };
    let mut buffer = Vec::new();
    match file.read_to_end(&mut buffer) {
        Ok(_) => Cursor::new(buffer),
        Err(err) => {
            panic!("Cannot read file '{}' ({})", full_path, err);
        }
    }
}

pub fn main() {
    set_load(Box::new(load_fn));

    let config = cmd_parse();
    if let Some(backend) = config.openxr {
        run_openxr(config, backend);
        return;
    }

    let config_poll = config.clone();

    let flow_count = config.flow_configs.len();

    let view_ports = match flow_count {
        1 => {
            vec![ViewPort {
                x: 0.0,
                y: 0.0,
                width: 1.0,
                height: 1.0,
                absolute_viewport: false,
            }]
        }
        2 => {
            vec![
                ViewPort {
                    x: 0.0,
                    y: 0.0,
                    width: 0.5,
                    height: 1.0,
                    absolute_viewport: false,
                },
                ViewPort {
                    x: 0.5,
                    y: 0.0,
                    width: 0.5,
                    height: 1.0,
                    absolute_viewport: false,
                },
            ]
        }
        _ => {
            panic!("Cannot create viewports for more than two flows")
        }
    };

    let mut frame_counter = 0;
    let mut frame_perfs: Vec<(u128, u128)> = vec![];
    let mut previous_frame = Instant::now();
    let print_spacing = 60;
    let output_processed: Arc<Mutex<Vec<Arc<RwLock<bool>>>>> = Arc::new(Mutex::new(Vec::new()));
    let output_processed_init = output_processed.clone();
    let output_processed_poll = output_processed.clone();

    let window = WindowSurface::new(
        config.visible,
        flow_count,
        config.flow_configs[0].static_gaze,
        move |surface| {
            for (index, flow_config) in config.flow_configs.iter().enumerate() {
                let mut io_generator = io::IoGenerator::new(
                    config.inputs.clone(),
                    flow_config.name.clone(),
                    config.output.clone(),
                );
                if config.output.is_some() {
                    output_processed_init
                        .lock()
                        .unwrap()
                        .push(io_generator.input_processed.clone());
                }
                build_flow(
                    surface,
                    &mut io_generator,
                    index,
                    config.resolution,
                    view_ports[index],
                    config.output_scale,
                );
            }

            let mut inspector = ConfigInspector::new(&config);
            surface.inspect(&mut inspector);
            inspector.print_unused();
        },
        move || {
            let mut done = false;
            frame_counter += 1;

            // Batch output and automatic exit should happen after ~3 frames to ensure proper/stable results.
            if config_poll.output.is_some() {
                let processed = output_processed_poll.lock().unwrap();
                done = !processed.is_empty()
                    && processed.iter().all(|processed| *processed.read().unwrap());
            } else if !config_poll.visible {
                done = true;
            }

            if config_poll.measure_frames > 0 {
                let time_diff = previous_frame.elapsed().as_micros();
                let frame_perf = (frame_counter, time_diff);
                frame_perfs.push(frame_perf);

                if frame_counter > 0 && frame_counter % print_spacing == 0 {
                    let avg_fps: i32 = frame_perfs
                        [(frame_counter - print_spacing) as usize..frame_counter as usize]
                        .iter()
                        .map(|t| t.1 as i32)
                        .sum::<i32>()
                        / (print_spacing as i32);

                    println!("{:?} ≙ {}fps", frame_perf, 1_000_000 / (avg_fps));
                }
                previous_frame = Instant::now();
                if frame_counter > config_poll.measure_frames {
                    done = true;
                }
            }

            /*
                The above hack works only with still images.
                The original solution below has several problems:
                - it is only used for video
                - There needs to be an io generator for each eye to provide them with independent input
                - one io generator shoult be able to multtiplex its output to both eyes
                - if one generator is ready, to we already trigger the render step or do we wait for both?
            */

            // if io_generator.is_ready() {
            //     if let Some((input_node, output_node)) = io_generator.next(&window, None) {
            //         window.replace_node(0, input_node, 0);
            //         let output_node = if let Some(output_node) = output_node {
            //             output_node
            //         } else {
            //             Box::new(Passthrough::new(&window))
            //         };
            //         window.replace_node(window.nodes_len() - 2, output_node, 0);
            //         window.update_nodes();
            //     } else {
            // ...
            //     }
            // }

            done
        },
    );

    let _ = window.run_app();
}

#[cfg(feature = "openxr")]
fn run_openxr(config: Config, backend: OpenXrBackend) {
    if config.flow_configs.len() > 2 {
        eprintln!("OpenXR accepts at most two eye-based flow configs.");
        std::process::exit(1);
    }

    let backend = match backend {
        OpenXrBackend::Auto => vss_openxr::Backend::Auto,
        OpenXrBackend::Vulkan => vss_openxr::Backend::Vulkan,
        OpenXrBackend::Metal => vss_openxr::Backend::Metal,
    };
    let runtime = vss_openxr::Runtime::new(vss_openxr::RuntimeOptions {
        backend,
        loader_path: None,
    });
    if let Err(err) = runtime.run(move |context, views| {
        for view in views {
            let flow_config = config
                .flow_configs
                .get(view.eye_index)
                .unwrap_or(&config.flow_configs[0]);
            let mut io_generator = io::IoGenerator::new(
                config.inputs.clone(),
                flow_config.name.clone(),
                config.output.clone(),
            );
            let viewport = ViewPort {
                x: view.viewport.x as f32,
                y: view.viewport.y as f32,
                width: view.viewport.width as f32,
                height: view.viewport.height as f32,
                absolute_viewport: true,
            };
            build_flow(
                context,
                &mut io_generator,
                view.view_index,
                config.resolution,
                viewport,
                config.output_scale,
            );
        }
    }) {
        eprintln!("{err}");
        std::process::exit(1);
    }
}

#[cfg(not(feature = "openxr"))]
fn run_openxr(_config: Config, _backend: OpenXrBackend) {
    eprintln!(
        "OpenXR support is not enabled. Rebuild with: cargo run -p vss-desktop --features openxr -- --openxr ..."
    );
    std::process::exit(1);
}
