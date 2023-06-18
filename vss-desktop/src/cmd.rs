use glob::glob;
use serde::Serialize;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::fs::File;
use std::io::prelude::*;
use vss::*;

#[derive(Serialize)]
pub struct OutputInfo {
    // Name of the configuration (e.g., `astigmatism`)
    pub configname: String,
    /// Path without basename, e.g., `path/to` (drops suffix after last `/`).
    pub dirname: String,
    /// Path without dirname, e.g., `image.png` (drops prefix up to last `/`).
    pub basename: String,
    /// Basename without file extension, e.g., `image` (excluding dot).
    pub stem: String,
    // Basename without stem, e.g., `png` (excluding dot).
    pub extension: String,
}

#[derive(Debug, Clone)]
pub struct FlowConfig {
    pub name: String,
    pub values: HashMap<String, serde_json::value::Value>,
    pub static_gaze: Option<(f32, f32)>,
    pub static_view: Option<(f32, f32)>,
}

#[derive(Debug, Clone)]
pub struct Config {
    pub visible: bool,
    pub resolution: Option<(u32, u32)>,
    pub measure_frames: u128,
    pub flow_configs: Vec<FlowConfig>,
    pub output: Option<mustache::Template>,
    pub output_scale: OutputScale,
    pub inputs: Vec<String>,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            visible: false,
            resolution: None,
            output: None,
            inputs: Vec::new(),
            flow_configs: vec![FlowConfig {
                name: "Default".to_string(),
                values: HashMap::new(),
                static_gaze: None,
                static_view: None,
            }],
            measure_frames: 0,
            output_scale: OutputScale::default(),
        }
    }
}

pub struct ConfigInspector<'a> {
    config: &'a Config,
    keys_used: RefCell<HashSet<String>>,
    flow_index: RefCell<usize>,
}

impl<'a> ConfigInspector<'a> {
    pub fn new(config: &'a Config) -> Self {
        ConfigInspector {
            config,
            keys_used: RefCell::new(HashSet::new()),
            flow_index: RefCell::new(usize::default()),
        }
    }

    pub fn print_unused(&self) {
        for flow_config in self.config.flow_configs.iter() {
            for (key, value) in flow_config.values.iter() {
                if !self.keys_used.borrow().contains(key) {
                    println!(
                        "Unused setting from {}: \"{}\" = \"{}\"",
                        flow_config.name, key, value
                    );
                }
            }
        }
    }
}

impl Inspector for ConfigInspector<'_> {
    fn flow(&self, index: usize, flow: &Flow) {
        self.flow_index.replace(index);
        flow.inspect(self);
        self.flow_index.take();
    }

    fn mut_node(&self, node: &mut dyn Node) {
        node.inspect(self);
    }

    fn mut_bool(&self, name: &'static str, value: &mut bool) -> bool {
        let config_values = &self.config.flow_configs[*self.flow_index.borrow()].values;
        if let Some(serde_json::value::Value::Bool(config_value)) = config_values.get(name) {
            *value = *config_value;
            self.keys_used.borrow_mut().insert(name.to_string());
            true
        } else {
            false
        }
    }

    fn mut_f64(&self, name: &'static str, value: &mut f64) -> bool {
        let config_values = &self.config.flow_configs[*self.flow_index.borrow()].values;
        if let Some(serde_json::value::Value::Number(config_value)) = config_values.get(name) {
            *value = config_value.as_f64().unwrap();
            self.keys_used.borrow_mut().insert(name.to_string());
            true
        } else {
            false
        }
    }

    fn mut_f32(&self, name: &'static str, value: &mut f32) -> bool {
        let config_values = &self.config.flow_configs[*self.flow_index.borrow()].values;
        if let Some(serde_json::value::Value::Number(config_value)) = config_values.get(name) {
            *value = config_value.as_f64().unwrap() as f32;
            self.keys_used.borrow_mut().insert(name.to_string());
            true
        } else {
            false
        }
    }

    fn mut_i32(&self, name: &'static str, value: &mut i32) -> bool {
        let config_values = &self.config.flow_configs[*self.flow_index.borrow()].values;
        if let Some(serde_json::value::Value::Number(config_value)) = config_values.get(name) {
            *value = config_value.as_f64().unwrap() as i32;
            self.keys_used.borrow_mut().insert(name.to_string());
            true
        } else {
            false
        }
    }

    fn mut_u32(&self, name: &'static str, value: &mut u32) -> bool {
        let config_values = &self.config.flow_configs[*self.flow_index.borrow()].values;
        if let Some(serde_json::value::Value::Number(config_value)) = config_values.get(name) {
            *value = config_value.as_f64().unwrap() as u32;
            self.keys_used.borrow_mut().insert(name.to_string());
            true
        } else {
            false
        }
    }

    fn mut_img(&self, name: &'static str, value: &mut String) -> bool {
        let config_values = &self.config.flow_configs[*self.flow_index.borrow()].values;
        if let Some(serde_json::value::Value::String(config_value)) = config_values.get(name) {
            *value = config_value.to_string();
            self.keys_used.borrow_mut().insert(name.to_string());
            true
        } else {
            false
        }
    }

    fn mut_matrix(&self, _name: &'static str, _value: &mut cgmath::Matrix4<f32>) -> bool {
        false //TODO: do this properly
    }
}

fn parse_flow_config(json_or_path: &str) -> Result<FlowConfig, Box<dyn Error>> {
    // Parse from string or as file.
    let (root, name): (serde_json::Value, String) =
        if let Ok(json) = serde_json::from_str(json_or_path) {
            (json, "custom string".to_string())
        } else {
            let mut data = String::new();
            let mut file = File::open(json_or_path)?;
            file.read_to_string(&mut data)?;
            (
                serde_json::from_str(&data)?,
                std::path::Path::new(json_or_path)
                    .file_stem()
                    .unwrap()
                    .to_os_string()
                    .into_string()
                    .unwrap(),
            )
        };
    let values = root.as_object().unwrap().clone().into_iter().collect();
    Ok(FlowConfig {
        name,
        values,
        static_gaze: None,
        static_view: None,
    })
}

pub fn cmd_parse() -> Config {
    use clap::{Arg, ArgAction, Command};

    let matches = Command::new("Visual System Simulator (VSS)")
        .version("1.1.0")
        .author("The Visual System Simulator Developers")
        .about("Simulates various aspects of the human visual system")
        .arg(
            Arg::new("show")
                .long("show")
                .short('s')
                .action(ArgAction::SetTrue)
                .help("Forces window visibility"),
        )
        .arg(
            Arg::new("res")
                .long("res")
                .value_names(["W", "H"])
                .num_args(2)
                .help("Sets the internal render resolution"),
        )
        .arg(
            Arg::new("measure_frames")
                .long("measure_frames")
                .num_args(1)
                .help("Tracks performance metrics for N frames"),
        )
        .arg(
            Arg::new("flow_configs")
                .long("flow_configs")
                .short('c')
                .value_name("FILE|JSON")
                .num_args(1)
                .action(ArgAction::Append)
                .help("Sets the configuration for a simulation flow"),
        )
        .arg(
            Arg::new("gaze")
                .long("gaze")
                .short('g')
                .value_names(["X", "Y"])
                .num_args(2)
                .help("Sets the fallback gaze position"),
        )
        .arg(
            Arg::new("view")
                .long("view")
                .value_names(["X", "Y"])
                .num_args(2)
                .help("Sets the fallback view position"),
        )
        .arg(
            Arg::new("output")
                .long("output")
                .short('o')
                .value_name("MUSTACHE_PATTERN?")
                .help(
                    "Enables output with optional mustache-style pattern, e.g.:\n\
                    \x20\x20\"{{dirname}}/{{stem}}_{{configname}}.{{extension}}\"  (default)\n\
                    \x20\x20\"{{dirname}}/{{configname}}_{{basename}}\"",
                ),
        )
        .arg(
            Arg::new("output_scale")
                .long("output_scale")
                .num_args(1)
                .help(
                    "Which Scaling should be used for the Display Node\n\
                \x20\x20available are: \"fit\", \"fill\" and \"stretch\"",
                ),
        )
        .arg(
            Arg::new("input")
                .value_name("INPUT|GLOB_PATTERN")
                .help(
                    "Input identifier or glob-style patterns, e.g.:\n\
                    \x20\x20image.png\n\
                    \x20\x20**/*.png video.avi",
                )
                .required(true)
                .action(ArgAction::Append)
                .index(1),
        )
        .arg_required_else_help(true)
        .get_matches();

    let mut config = Config::default();

    config.visible = matches.get_flag("show");

    let res = matches
        .get_many::<String>("res")
        .unwrap_or_default()
        .map(|t| t.trim().parse::<u32>().unwrap())
        .collect::<Vec<u32>>();
    if res.len() == 2 {
        config.resolution = Some((res[0], res[1]));
    }

    config.measure_frames = matches
        .get_one::<String>("measure_frames")
        .map_or(config.measure_frames, |v| {
            v.parse::<u128>().unwrap_or(0u128)
        });

    let flow_configs = matches
        .get_many::<String>("flow_configs")
        .unwrap_or_default()
        .map(|config_str| parse_flow_config(config_str.as_str()).expect("Invalid flow config"))
        .collect::<Vec<FlowConfig>>();
    if !flow_configs.is_empty() {
        config.flow_configs = flow_configs;
    }

    let gaze = matches
        .get_many::<String>("gaze")
        .unwrap_or_default()
        .map(|t| t.trim().parse::<f32>().unwrap())
        .collect::<Vec<f32>>();
    if gaze.len() == 2 {
        for flow_config in &mut config.flow_configs {
            flow_config.static_gaze = Some((gaze[0], gaze[1]));
        }
    }

    let view = matches
        .get_many::<String>("view")
        .unwrap_or_default()
        .map(|t| t.trim().parse::<f32>().unwrap())
        .collect::<Vec<f32>>();
    if view.len() == 2 {
        for flow_config in &mut config.flow_configs {
            flow_config.static_view = Some((view[0], view[1]));
        }
    }

    if matches.contains_id("output") {
        let output = if let Some(output) = matches.get_one::<String>("output") {
            output
        } else {
            "{{dirname}}/{{stem}}_{{configname}}.{{extension}}"
        };
        config.output = Some(mustache::compile_str(output).unwrap());
    } else {
        config.visible = true;
    }

    config.output_scale = if let Some(str) = matches.get_one::<String>("output_scale") {
        OutputScale::from_string(str)
    } else {
        OutputScale::default()
    };

    config.inputs = matches
        .get_many::<String>("input")
        .unwrap_or_default()
        .flat_map(|pattern| {
            if let Ok(entries) = glob(pattern) {
                entries
                    .map(|entry| entry.unwrap().into_os_string().into_string().unwrap())
                    .collect::<Vec<String>>()
            } else {
                vec![pattern.to_string()]
            }
        })
        .collect();

    config
}
