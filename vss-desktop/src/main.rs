mod cmd;
mod node;
#[cfg(feature = "varjo")]
mod varjo;

#[cfg(feature = "openxr")]
mod openxr;

use std::cell::RefCell;
use std::time::Instant;
use std::fs;
use std::fs::OpenOptions;
use vss::*;

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

    #[cfg(feature = "varjo")]
    fn is_ready(&self) -> bool {
        *self.input_processed.read().unwrap()
    }

    #[cfg(feature = "varjo")]
    fn next(&mut self, window: &Window, render_resolution: Option<[u32; 2]>) -> Option<IoNodePair> {
        self.input_idx += 1;
        self.current(&window, render_resolution,0)
    }

    fn current(&mut self, window: &Window, render_resolution: Option<[u32; 2]>, flow_index: usize) -> Option<IoNodePair> {
        if self.input_idx >= self.inputs.len() {
            None
        } else {
            let input = &self.inputs[self.input_idx];
            if UploadRgbBuffer::has_image_extension(&input) {
                let input_path = std::path::Path::new(input);
                let mut input_node = UploadRgbBuffer::new(&window);
                input_node.upload_image(load(input_path));
                input_node.set_flags(RgbInputFlags::from_extension(&input));
                input_node.set_render_resolution(render_resolution);
                let output_node = if let Some(output) = &self.output {
                    let mut output_node = DownloadRgbBuffer::new(&window);
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

fn build_flow(window: &mut Window, io_generator: &mut IoGenerator, flow_index: usize, render_resolution: Option<[u32; 2]>, variance_log: Option<String>){
    let (input_node, output_node) = io_generator.current(&window, render_resolution, flow_index).unwrap();

    // Add input node.
    window.add_node(input_node, flow_index);

    // Visual system passes.
    let node = Cataract::new(&window);
    window.add_node(Box::new(node), flow_index);
    let node = Lens::new(&window);
    window.add_node(Box::new(node), flow_index);
    let node = Retina::new(&window);
    window.add_node(Box::new(node), flow_index);
    let node = PeacockCB::new(&window);
    window.add_node(Box::new(node), flow_index);
    
    // Measure Uncertainty
    let mut node = VarianceMeasure::new(&window);
    if variance_log.is_some(){
        node.log_file = Some(OpenOptions::new()
            .write(true)
            .append(true)
            .create(true)
            .open(variance_log.unwrap())
            .unwrap());
    }
    window.add_node(Box::new(node), flow_index);
    
    // Add output node, if present.
    if let Some(output_node) = output_node {
        // Display node.
        let node = Display::new(&window);
        window.add_node(Box::new(node), flow_index);
        window.add_node(output_node, flow_index);
    } else {
        // window.add_node(Box::new(Passthrough::new(&window)), flow_index)
        // Display node.
        let node = Display::new(&window);
        window.add_node(Box::new(node), flow_index);
    }


}

#[cfg(not(any(feature = "varjo", feature = "openxr")))]
pub fn main() {
    let config = cmd_parse();

    // let remote = if let Some(port) = config.port {
    //     Some(Remote::new(port))
    // } else {
    //     None
    // };
    
    let flow_count = match (config.parameters_r.clone(), config.parameters_l.clone()) {
        (Some(_), Some(_)) => 2 , 
        _ => 1
    };

    let mut parameters = Vec::new();
    for idx in 0 .. flow_count {
        let mut value_map = ValueMap::new();
        let iter = match (config.parameters_r.clone(), config.parameters_l.clone()) {
            (Some(param_r), Some(param_l)) =>{
                if idx == 0 {
                    param_r.into_iter()
                }
                else{
                    param_l.into_iter()
                }
            }
            _ => {
                config.parameters.clone().into_iter()
            }
        };
        for (key, val) in iter {
            value_map.insert((key).clone(), (val).clone());
        }  
        value_map.insert("flow_id".into(),Value::Number(idx as f64));
        parameters.push(RefCell::new(value_map));
    }

    let mut window = Window::new(config.visible, None, parameters, flow_count);

    let is_output_hack_used = config.output.is_some();

    // let viewports = vec![
    //     (0, 0, 640, 360),
    //     (640, 0, 640, 360)
    // ];

    let mut desktop = SharedStereoDesktop::new();

    for index in 0..flow_count {
        let mut io_generator = IoGenerator::new(
            config.inputs.clone(),
            config.name.clone(),
            config.output.clone(),
        );
        build_flow(&mut window, &mut io_generator, index, config.resolution, config.variance_log.clone());

        if flow_count > 1 {
            let node = desktop.get_stereo_desktop_node(&window);
            window.add_node(Box::new(node), index);
        }
    }
    

    let mut done = false;
    window.update_last_node();
    window.update_nodes();

    let mut frame_counter = 0u128; // you know, for the simulations than run longer than the universe exists

    let mut frame_times: Vec<(u128,u128,(u32,u32),u32,u32)> = vec![];
    let mut previous_frame = Instant::now();
    let print_spacing = 60;
    let rays = if let Some(Value::Number(rays)) = config.parameters.get("rays") {
        *rays as u32
    }else{0};

    let mix_type = if let Some(Value::Number(mix_type)) = config.parameters.get("file_mix_type") {
        *mix_type as u32
    }else{0};

    while !done {
        frame_counter+=1;

        done = window.poll_events();

        if !config.visible || is_output_hack_used && frame_counter == 3 {
            // Exit once all inputs have been processed, unless visible.
            done = true;
        }

        if config.track_perf >0 {
            let time_diff = previous_frame.elapsed().as_micros();
            let perf_info = (frame_counter,time_diff, (config.resolution.unwrap()[0],config.resolution.unwrap()[1]), rays, mix_type);
            frame_times.push(perf_info);

            if frame_counter>0 && frame_counter % print_spacing == 0 {
                let avg_fps: i32 = frame_times[(frame_counter-print_spacing) as usize .. frame_counter as usize]
                .iter()
                .map(|t| t.1 as i32)
                .sum::<i32>() / (print_spacing as i32);

                println!("{:?} ≙ {}fps", perf_info, 1_000_000/(avg_fps));
            }
            previous_frame =  Instant::now();
            if frame_counter > config.track_perf as u128 {
                break;
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
    }

    // writing the image to file might not be done yet, so we wait a second
    // this async behaviour stems from the callback used in the download buffer
    if config.track_perf > 0 {
        dump_perf_data(frame_times);
    }
    use std::{thread, time};
    let a_second = time::Duration::from_secs(1);
    thread::sleep(a_second);
}

fn dump_perf_data(frame_times: Vec<(u128,u128,(u32,u32),u32,u32)>){
    match fs::write("vss_perf_data.csv", 
        frame_times.iter()
            .map(|t| format!("{},{},{},{},{}\n", t.0, t.1, t.2.0*t.2.1, t.3, t.4))
            .collect::<Vec<String>>()
            .join("")){
                Err(e) => println!("dump_perf_data error {:?}", e),
                _ => ()
    }
}

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
    for idx in 0 .. flow_count {
        let mut value_map = ValueMap::new();
        let iter = match (config.parameters_r.clone(), config.parameters_l.clone()) {
            (Some(param_r), Some(param_l)) =>{
                if idx == 0 {
                    param_r.into_iter()
                }
                else{
                    param_l.into_iter()
                }
            }
            _ => {
                config.parameters.clone().into_iter()
            }
        };
        for (key, val) in iter {
            value_map.insert((key).clone(), (val).clone());
        }  
        value_map.insert("flow_id".into(),Value::Number(idx as f64));
        parameters.push(RefCell::new(value_map));
    }

    let mut window = Window::new(config.visible, remote, parameters, flow_count);

    
    dbg!("Pre init");
    oxr.initialize();
    dbg!("Post init");




    let mut desktop = SharedStereoDesktop::new();

    for index in 0..flow_count {
        let mut io_generator = IoGenerator::new(
            config.inputs.clone(),
            config.name.clone(),
            config.output.clone(),
        );

        build_flow(&mut window, &mut io_generator, index, config.resolution);
        let mut node = desktop.get_stereo_desktop_node(&window);

        window.add_node(Box::new(node), index);
    }


    let mut done = false;
    window.update_last_node();

    oxr.create_session(&window);
    oxr.create_render_targets(&window);


    while !done {
        done = window.poll_events();
    }
}

#[cfg(feature = "varjo")]
pub fn set_varjo_data(window: &mut Window, last_fov: &mut Vec<(f32, f32)>, varjo: &mut varjo::Varjo){
    let (varjo_target_color, varjo_target_depth) = varjo.get_current_render_target();
    window.replace_targets(varjo_target_color, varjo_target_depth, false);

    let view_matrices = varjo.get_current_view_matrices();
    let proj_matrices = varjo.get_current_proj_matrices();
    let head_position = 0.5 * (view_matrices[0].w.truncate() + view_matrices[1].w.truncate());
    let (left_gaze, right_gaze, _focus_distance) = varjo.get_current_gaze();

    for index in 0 .. 4 {
        let fov_x = 2.0*(1.0/proj_matrices[index][0][0]).atan();// * 180.0 / 3.1415926535;
        let fov_y = 2.0*(1.0/proj_matrices[index][1][1]).atan();// * 180.0 / 3.1415926535;
        if last_fov[index].0 != fov_x || last_fov[index].1 != fov_y {
            window.set_value("fov_x".to_string(), Value::Number(fov_x as f64), index);
            window.set_value("fov_y".to_string(), Value::Number(fov_y as f64), index);
            window.set_value("proj_matrix".to_string(), Value::Matrix(proj_matrices[index%2]), index);
            last_fov[index].0 = fov_x;
            last_fov[index].1 = fov_y;
        }
        window.set_perspective(EyePerspective{
            position: head_position,
            view: view_matrices[index],
            proj: proj_matrices[index],
            gaze: if index%2 == 0 {left_gaze} else {right_gaze},
        },index);
    }
}

#[cfg(feature = "varjo")]
pub fn main() {
    let config = cmd_parse();

    let remote = if let Some(port) = config.port {
        Some(Remote::new(port))
    } else {
        None
    };

    let flow_count = 4; //TODO take this number from the varjo api instead ? (for example the size of the viewports vector)
    let mut parameters = Vec::new();
    for _ in 0 .. flow_count {
        let mut value_map = ValueMap::new();
        for (key, val) in config.parameters.iter() {
            value_map.insert((*key).clone(), (*val).clone());
        }
        parameters.push(RefCell::new(value_map));
    }

    let mut window = Window::new(config.visible, remote, parameters, flow_count);

    let mut varjo = varjo::Varjo::new();
    let mut log_counter = 0; //TODO: used to reduce log spam, remove when no longer needed or replace with a better solution
    let varjo_viewports = varjo.create_render_targets(&window);
    let mut varjo_fov = vec![(100.0, 70.0); 4];

    let mut io_generator = IoGenerator::new(config.inputs, config.name.clone(), config.output);

    for index in 0 .. flow_count {
        let viewport = &varjo_viewports[index];
        build_flow(&mut window, &mut io_generator, index, Some([viewport.width, viewport.height]), None);
        let mut node = VRCompositor::new(&window);
        node.set_viewport(viewport.x as f32, viewport.y as f32, viewport.width as f32, viewport.height as f32);
        window.add_node(Box::new(node), index);
    }

    let mut done = false;
    while !done {
        varjo.logging_enabled = log_counter == 0;

        let varjo_should_render = varjo.begin_frame_sync();

        if varjo_should_render {
            set_varjo_data(&mut window, &mut varjo_fov, &mut varjo);
        }
        
        window.update_last_node();
        
        done = window.poll_events();

        if varjo_should_render {
            varjo.end_frame();
            log_counter = (log_counter+1) % 10;
        }

        if io_generator.is_ready() {
            if let Some((input_node, output_node)) = io_generator.next(&window, None) {
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
