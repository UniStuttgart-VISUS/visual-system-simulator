mod cmd;
mod node;

#[cfg(feature = "openxr")]
mod openxr;

use std::io::Cursor;
use std::time::Instant;
use vss::*;
#[cfg(not(any(feature = "varjo", feature = "openxr")))]
use vss_winit::*;
#[cfg(feature = "varjo")]
use vss_vr::*;

use crate::cmd::*;
use crate::node::*;

type IoNodePair = (Box<dyn Node>, Option<Box<dyn Node>>);

struct IoGenerator {
    inputs: Vec<String>,
    config_name: String,
    output: Option<mustache::Template>,
    input_idx: usize,
    input_processed: std::sync::Arc<std::sync::RwLock<bool>>,
}

impl IoGenerator {
    fn new(inputs: Vec<String>, config_name: String, output: Option<mustache::Template>) -> Self {
        Self {
            inputs,
            config_name,
            output,
            input_idx: 0,
            input_processed: std::sync::Arc::new(std::sync::RwLock::new(false)),
        }
    }

    fn _is_ready(&self) -> bool {
        *self.input_processed.read().unwrap()
    }

    fn _next(
        &mut self,
        surface: &Surface,
        render_resolution: Option<[u32; 2]>,
    ) -> Option<IoNodePair> {
        self.input_idx += 1;
        let render_res = if let Some(res) = render_resolution {
            RenderResolution::Custom { res }
        } else {
            RenderResolution::Buffer { input_scale: 1.0 } //TODO add input scaling
        };
        self.current(surface, render_res, 0)
    }

    fn current(
        &mut self,
        surface: &Surface,
        render_resolution: RenderResolution,
        flow_index: usize,
    ) -> Option<IoNodePair> {
        if self.input_idx >= self.inputs.len() {
            None
        } else {
            let input = &self.inputs[self.input_idx];
            if UploadRgbBuffer::has_image_extension(input) {
                let input_path = std::path::Path::new(input);
                let mut input_node = UploadRgbBuffer::new(surface);
                input_node.upload_image(load(input_path));
                input_node.set_flags(
                    RgbInputFlags::from_extension(input) | RgbInputFlags::VERTICALLY_FLIPPED,
                );
                input_node.set_render_resolution(render_resolution);
                let output_node = if let Some(output) = &self.output {
                    let mut output_node = DownloadRgbBuffer::new(surface);
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
                let mut input_node = UploadVideo::new(surface);
                input_node.set_flags(RgbInputFlags::from_extension(input));
                input_node.open(input).unwrap();
                Some((Box::new(input_node), None))
            } else {
                panic!("Unknown file extension");
            }
        }
    }
}

#[cfg(not(any(feature = "varjo", feature = "openxr")))]
fn build_flow(
    surface: &mut Surface,
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
        .current(surface, render_res, flow_index)
        .unwrap();

    // Add input node.
    surface.add_node(input_node, flow_index);

    // Visual system passes.
    let node = Cataract::new(surface);
    surface.add_node(Box::new(node), flow_index);
    let node = Lens::new(surface);
    surface.add_node(Box::new(node), flow_index);
    let node = Retina::new(surface);
    surface.add_node(Box::new(node), flow_index);
    let node = PeacockCB::new(surface);
    surface.add_node(Box::new(node), flow_index);

    // Measurement Nodes for variance and error
    let node = VarianceMeasure::new(surface);
    surface.add_node(Box::new(node), flow_index);
    let node = VisOverlay::new(surface);
    surface.add_node(Box::new(node), flow_index);

    // Display node.
    let mut node = Display::new(surface);
    node.set_viewport(view_port);
    node.set_output_scale(output_scale);
    surface.add_node(Box::new(node), flow_index);

    // Add UI overlay.
    let node = GuiOverlay::new(surface);
    surface.add_node(Box::new(node), flow_index);

    // Add output node, if present.
    if let Some(output_node) = output_node {
        surface.add_node(output_node, flow_index);
    }

    surface.negociate_slots();
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

// "Default" main
#[cfg(not(any(feature = "varjo", feature = "openxr")))]
pub fn main() {
    set_load(Box::new(load_fn));

    let config = cmd_parse();
    let config_poll = config.clone();

    let flow_count = config.flow_configs.len();

    let window = WindowSurface::new(
        config.visible,
        flow_count,
        config.flow_configs[0].static_gaze,
    );

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

    pollster::block_on(window.run_and_exit(
        move |surface| {
            for (index, flow_config) in config.flow_configs.iter().enumerate() {
                let mut io_generator = IoGenerator::new(
                    config.inputs.clone(),
                    flow_config.name.clone(),
                    config.output.clone(),
                );
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
            if !config_poll.visible || config_poll.output.is_some() && frame_counter == 3 {
                // Exit once all inputs have been processed, unless visible.
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
                The above hack works only with still images
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
    ));

    /*

    if config.measure_frames > 0 {
        if let Err(e) = fs::write(
            "vss_perf_data.csv",
            frame_perfs
                .iter()
                .map(|t| format!("{},{}\n", t.0, t.1))
                .collect::<Vec<String>>()
                .join(""),
        ) {
            println!("dump_perf_data error {:?}", e);
        }
    }

    // writing the image to file might not be done yet, so we wait a second
    // this async behaviour stems from the callback used in the download buffer
    std::thread::sleep(std::time::Duration::from_secs(1));
    */
}

//TODO: this one is super unfinished.
#[cfg(feature = "openxr")]
pub fn main() {
    let mut oxr = openxr::OpenXR::new();
    let config = cmd_parse();

    let remote = if let Some(port) = config.port {
        Some(Remote::new(port))
    } else {
        None
    };

    let flow_count = 2;

    let mut parameters = Vec::new();
    for idx in 0..flow_count {
        let mut value_map = ValueMap::new();
        let iter = match (config.parameters_r.clone(), config.parameters_l.clone()) {
            (Some(param_r), Some(param_l)) => {
                if idx == 0 {
                    param_r.into_iter()
                } else {
                    param_l.into_iter()
                }
            }
            _ => config.parameters.clone().into_iter(),
        };
        for (key, val) in iter {
            value_map.insert((key).clone(), (val).clone());
        }
        value_map.insert("flow_id".into(), Value::Number(idx as f64));
        parameters.push(RefCell::new(value_map));
    }

    let mut window = Window::new(config.visible, remote, parameters, flow_count);

    dbg!("Pre init");
    oxr.initialize();
    dbg!("Post init");

    for index in 0..flow_count {
        let mut io_generator = IoGenerator::new(
            config.inputs.clone(),
            config.name.clone(),
            config.output.clone(),
        );

        build_flow(
            &mut window.surface,
            &mut io_generator,
            index,
            config.resolution,
        );
    }

    let mut done = false;

    oxr.create_session(&window);
    oxr.create_render_targets(&window);

    while !done {
        done = window.poll_events();
    }
}

#[cfg(feature = "varjo")]
fn build_vr_flow(
    vr_surface: &mut WindowVRSurface,
    surface: &mut Surface,
    io_generator: &mut IoGenerator,
    flow_index: usize,
    render_resolution: Option<(u32, u32)>,
    view_port: ViewPort,
) {
    let render_res = if let Some(res) = render_resolution {
        RenderResolution::Custom {
            res: [res.0, res.1],
        }
    } else {
        RenderResolution::Screen {
            input_scale: 1.0, //TODO add input scaling
            output_scale: OutputScale::Stretch,
        }
    };
    let (input_node, output_node) = io_generator
        .current(surface, render_res, flow_index)
        .unwrap();

    // Add input node.
    vr_surface.add_node(input_node, flow_index);

    // Visual system passes.
    let node = Cataract::new(surface);
    vr_surface.add_node(Box::new(node), flow_index);
    let node = Lens::new(surface);
    vr_surface.add_node(Box::new(node), flow_index);
    let node = Retina::new(surface);
    vr_surface.add_node(Box::new(node), flow_index);
    let node = PeacockCB::new(surface);
    vr_surface.add_node(Box::new(node), flow_index);

    // Display node.
    let mut node = Display::new(surface);
    node.set_viewport(view_port);
    node.set_output_scale(OutputScale::Stretch);
    vr_surface.add_node(Box::new(node), flow_index);

    // Add output node, if present.
    if let Some(output_node) = output_node {
        vr_surface.add_node(output_node, flow_index);
    }

    vr_surface.negociate_slots(&surface);
}

#[cfg(feature = "varjo")]
fn build_window_flow(
    surface: &mut Surface,
    flow_index: usize,
    input_texture: Texture,
    view_port: ViewPort,
    output_scale: OutputScale,
) {
    // Add input node.
    let node = VrBuffer::new(surface, input_texture, None);
    surface.add_node(Box::new(node), flow_index);

    // Measurement Nodes for variance and error
    let node = VarianceMeasure::new(surface);
    surface.add_node(Box::new(node), flow_index);
    // TODO: the VrBuffer node currently can't access these values.
    // But if there is an interest in it, it should be possible to add these textures too.
    // let node = VisOverlay::new(surface);
    // surface.add_node(Box::new(node), flow_index);

    // Display node.
    let mut node = Display::new(surface);
    node.set_viewport(view_port);
    node.set_output_scale(output_scale);
    surface.add_node(Box::new(node), flow_index);

    // Add UI overlay.
    let node = GuiOverlay::new(surface);
    surface.add_node(Box::new(node), flow_index);

    surface.negociate_slots();
}

#[cfg(feature = "varjo")]
pub fn main() {
    let varjo = Varjo::new();

    let (varjo_viewports, varjo_texture_width, varjo_texture_height) = varjo.get_viewports();

    set_load(Box::new(load_fn));

    let config = cmd_parse();
    let config_poll = config.clone();

    let flow_count = varjo_viewports.len();
    assert!(flow_count == config.flow_configs.len(), "Number of provided configs does not match viewport count of {}", flow_count);

    let view_ports = varjo_viewports.iter().map( |vp| {
        ViewPort{
            x: vp.x as f32 / varjo_texture_width as f32,
            y: 1.0 - ((vp.y + vp.height) as f32 / varjo_texture_height as f32), // flip on the y axis
            width: vp.width as f32 / varjo_texture_width as f32,
            height: vp.height as f32 / varjo_texture_height as f32,
            absolute_viewport: false,
        }
    }).collect::<Vec<ViewPort>>();

    let window_view_port = ViewPort {
        x: 0.0,
        y: 0.0,
        width: 1.0,
        height: 1.0,
        absolute_viewport: false,
    };

    let window = WindowVRSurface::new(
        config.visible,
        flow_count,
        config.flow_configs[0].static_gaze,
        varjo,
    );

    let mut frame_counter = 0;
    let mut frame_perfs: Vec<(u128, u128)> = vec![];
    let mut previous_frame = Instant::now();
    let print_spacing = 60;

    pollster::block_on(window.run_and_exit(
        move |vr_surface, surface, vr_framebuffer_texture| {
            
            for (index, flow_config) in config.flow_configs.iter().enumerate() {
                let mut io_generator = IoGenerator::new(
                    config.inputs.clone(),
                    flow_config.name.clone(),
                    config.output.clone(),
                );
                build_vr_flow(
                    vr_surface,
                    surface,
                    &mut io_generator,
                    index,
                    config.resolution,
                    view_ports[index],
                );
            }

            build_window_flow(
                surface,
                0,
                vr_framebuffer_texture,
                window_view_port,
                config.output_scale,
            );

            let mut inspector = ConfigInspector::new(&config);
            vr_surface.inspect(&mut inspector);
            inspector.print_unused();
        },
        move || {
            let mut done = false;
            frame_counter += 1;

            // Batch output and automatic exit should happen after ~3 frames to ensure proper/stable results.
            if !config_poll.visible || config_poll.output.is_some() && frame_counter == 3 {
                // Exit once all inputs have been processed, unless visible.
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
                The above hack works only with still images
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
        }));
}
