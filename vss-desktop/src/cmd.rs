use glob::glob;
use mustache;
use serde::Serialize;
use std::collections::HashMap;
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
    pub port: Option<u16>,
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
            port: None,
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
    flow_index: usize,
}

impl<'a> ConfigInspector<'a> {
    pub fn new(config: &'a Config) -> Self {
        ConfigInspector {
            config,
            flow_index: 0,
        }
    }
}

impl Inspector for ConfigInspector<'_> {
    fn begin_flow(&mut self, index: usize) {
        dbg!(index);
        self.flow_index = index;
    }

    fn end_flow(&mut self) {
        self.flow_index = 0;
    }

    fn begin_node(&mut self, name: &'static str) {
        dbg!(name);
    }

    fn end_node(&mut self) {}

    fn mut_bool(&mut self, name: &'static str, value: &mut bool) -> bool {
        let config_values = &self.config.flow_configs[self.flow_index].values;
        if let Some(serde_json::value::Value::Bool(config_value)) = config_values.get(name) {
            *value = *config_value;
            true
        } else {
            false
        }
    }

    fn mut_f64(&mut self, name: &'static str, value: &mut f64) -> bool {
        let config_values = &self.config.flow_configs[self.flow_index].values;
        if let Some(serde_json::value::Value::Number(config_value)) = config_values.get(name) {
            *value =  config_value.as_f64().unwrap();
            true
        } else {
            false
        }
    }

    fn mut_f32(&mut self, name: &'static str, value: &mut f32) -> bool {
        let config_values = &self.config.flow_configs[self.flow_index].values;
        if let Some(serde_json::value::Value::Number(config_value)) = config_values.get(name) {
            *value =  config_value.as_f64().unwrap() as f32;
            true
        } else {
            false
        }
    }

    fn mut_i32(&mut self, name: &'static str, value: &mut i32) -> bool {
        let config_values = &self.config.flow_configs[self.flow_index].values;
        if let Some(serde_json::value::Value::Number(config_value)) = config_values.get(name) {
            *value =  config_value.as_f64().unwrap() as i32;
            true
        } else {
            false
        }
    }

    fn mut_u32(&mut self, name: &'static str, value: &mut u32) -> bool {
        let config_values = &self.config.flow_configs[self.flow_index].values;
        if let Some(serde_json::value::Value::Number(config_value)) = config_values.get(name) {
            *value =  config_value.as_f64().unwrap() as u32;
            true
        } else {
            false
        }
    }

    fn mut_img(&mut self, name: &'static str, value: &mut String) -> bool {
        let config_values = &self.config.flow_configs[self.flow_index].values;
        if let Some(serde_json::value::Value::String(config_value)) = config_values.get(name) {
            *value =  config_value.to_string() ;
            true
        } else {
            false
        }
    }

    fn mut_matrix(&mut self, name: &'static str, value: &mut cgmath::Matrix4<f32>) -> bool {
    false //TODO: do this properly
    }
}

fn parse_flow_config(json_or_path: &str) -> Result<FlowConfig, Box<dyn Error>> {
    // Parse from string or as file.
    let (root, name): (serde_json::Value, String) =
        if let Ok(json) = serde_json::from_str(&json_or_path) {
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
    use clap::{App, AppSettings, Arg};

    let matches = App::new("Visual System Simulator (VSS)")
        .version("1.1.0")
        .author("The Visual System Simulator Developers")
        .about("Simulates various aspects of the human visual system")
        .arg(
            Arg::with_name("port")
                .long("port")
                .short("p")
                .value_name("PORT")
                .help("Specifies the port on which the server listens for connections"),
        )
        .arg(
            Arg::with_name("show")
                .long("show")
                .short("s")
                .takes_value(false)
                .help("Forces window visibility"),
        )
        .arg(
            Arg::with_name("res")
                .long("res")
                .value_names(&["W", "H"])
                .number_of_values(2)
                .help("Sets the internal render resolution"),
        )
        .arg(
            Arg::with_name("measure_frames")
                .long("measure_frames")
                .number_of_values(1)
                .help("Tracks performance metrics for N frames"),
        )
        .arg(
            Arg::with_name("flow_configs")
                .long("flow_configs")
                .short("c")
                .value_name("FILE|JSON")
                .number_of_values(1)
                .multiple(true)
                .help("Sets the configuration for a simulation flow"),
        )
        .arg(
            Arg::with_name("gaze")
                .long("gaze")
                .short("g")
                .value_names(&["X", "Y"])
                .number_of_values(2)
                .help("Sets the fallback gaze position"),
        )
        .arg(
            Arg::with_name("view")
                .long("view")
                .value_names(&["X", "Y"])
                .number_of_values(2)
                .help("Sets the fallback view position"),
        )
        .arg(
            Arg::with_name("output")
                .long("output")
                .short("o")
                .value_name("MUSTACHE_PATTERN?")
                .min_values(0)
                .max_values(1)
                .help(
                    "Enables output with optional mustache-style pattern, e.g.:\n\
                    \x20\x20\"{{dirname}}/{{stem}}_{{configname}}.{{extension}}\"  (default)\n\
                    \x20\x20\"{{dirname}}/{{configname}}_{{basename}}\"",
                ),
        )
        .arg(
            Arg::with_name("output_scale")
                .long("output_scale")
                .number_of_values(1)
                .help(
                    "Which Scaling should be used for the Display Node\n\
                \x20\x20available are: \"fit\", \"fill\" and \"stretch\"",
                ),
        )
        .arg(
            Arg::with_name("input")
                .value_name("INPUT|GLOB_PATTERN")
                .help(
                    "Input identifier or glob-style patterns, e.g.:\n\
                    \x20\x20image.png\n\
                    \x20\x20**/*.png video.avi",
                )
                .required(true)
                .multiple(true)
                .index(1),
        )
        .setting(AppSettings::ArgRequiredElseHelp)
        .setting(AppSettings::UnifiedHelpMessage)
        .get_matches();

    let mut config = Config::default();

    if let Some(port_str) = matches.value_of("port") {
        config.port = Some(port_str.parse::<u16>().expect("Invalid port"))
    }

    if matches.is_present("show") {
        config.visible = true;
    }

    if let Some(res) = matches.values_of("res") {
        let res = res
            .map(|t| t.trim().parse::<u32>().unwrap())
            .collect::<Vec<u32>>();
        config.resolution = Some((res[0], res[1]));
    };

    config.measure_frames = matches
        .value_of("measure_frames")
        .map(|v| v.parse::<u128>())
        .unwrap_or(Ok(0u128))
        .unwrap_or(config.measure_frames);

    if let Some(configs) = matches.values_of("flow_configs") {
        config.flow_configs = configs
            .map(|config_str| parse_flow_config(config_str).expect("Invalid flow config"))
            .collect()
    }

    if let Some(gaze) = matches.values_of("gaze") {
        let gaze = gaze
            .map(|t| t.trim().parse::<f32>().unwrap())
            .collect::<Vec<f32>>();
        for flow_config in &mut config.flow_configs {
            flow_config.static_gaze = Some((gaze[0], gaze[1]));
        }
    }

    if let Some(view) = matches.values_of("view") {
        let view = view
            .map(|t| t.trim().parse::<f32>().unwrap())
            .collect::<Vec<f32>>();
        for flow_config in &mut config.flow_configs {
            flow_config.static_view = Some((view[0], view[1]));
        }
    }

    if matches.is_present("output") {
        let output = if let Some(output) = matches.value_of("output") {
            output
        } else {
            "{{dirname}}/{{stem}}_{{configname}}.{{extension}}"
        };
        config.output = Some(mustache::compile_str(output).unwrap());
    } else {
        config.visible = true;
    }

    config.output_scale = if let Some(str) = matches.value_of("output_scale") {
        OutputScale::from_string(str)
    } else {
        OutputScale::default()
    };

    config.inputs = matches
        .values_of("input")
        .unwrap()
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
