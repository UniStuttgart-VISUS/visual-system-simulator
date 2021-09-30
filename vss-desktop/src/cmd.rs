use glob::glob;
use mustache;
use serde::Serialize;
use std::error::Error;
use std::fs::File;
use std::io::prelude::*;
use vss::*;

#[derive(Serialize)]
pub struct InputInfo {
    /// Path without basename, e.g., `path/to` (drops suffix after last `/`).
    pub dirname: String,
    /// Path without dirname, e.g., `image.png` (drops prefix up to last `/`).
    pub basename: String,
    /// Basename without file extension, e.g., `image` (excluding dot).
    pub stem: String,
    // Basename without stem, e.g., `png` (excluding dot).
    pub extension: String,
}

#[derive(Debug)]
pub struct Config {
    pub port: Option<u16>,
    pub visible: bool,
    pub resolution: Option<[u32; 2]>,
    pub output: Option<mustache::Template>,
    pub inputs: Vec<String>,
    pub parameters: ValueMap,
    pub parameters_r: Option<ValueMap>,
    pub parameters_l: Option<ValueMap>,
    pub track_perf: u32
}

impl Default for Config {
    fn default() -> Self {
        Config {
            port: Some(9009),
            visible: false,
            resolution: None,
            output: None,
            inputs: Vec::new(),
            parameters: std::collections::HashMap::new(),
            parameters_r: None,
            parameters_l: None,
            track_perf: 0
        }
    }
}

fn from_json(arg: &str) -> Result<serde_json::Value, Box<dyn Error>> {
    if let Ok(json) = serde_json::from_str(&arg) {
        Ok(json)
    } else {
        let mut data = String::new();
        let mut file = File::open(arg)?;
        file.read_to_string(&mut data)?;

        let json = serde_json::from_str(&data)?;
        Ok(json)
    }
}

fn from_json_obj(arg: &str) -> Result<ValueMap, Box<dyn Error>> {
    let mut map = ValueMap::new();
    let json = from_json(arg)?;
    let object = json.as_object().unwrap();

    for (key, value) in object.iter() {
        match value {
            serde_json::value::Value::Null => {}
            serde_json::value::Value::Bool(b) => {
                map.insert(key.to_string(), Value::Bool(*b));
            }
            serde_json::value::Value::Number(n) => {
                map.insert(key.to_string(), Value::Number(n.as_f64().unwrap()));
            }
            serde_json::value::Value::String(ref s) => {
                map.insert(key.to_string(), Value::Image(s.to_string()));
            }
            serde_json::value::Value::Array(_) => {}
            serde_json::value::Value::Object(_) => {}
        }
    }
    Ok(map)
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
            Arg::with_name("config")
                .long("config")
                .short("c")
                .value_name("FILE|JSON")
                .number_of_values(1)
                .help("Sets the configuration parameters for simulation"),
        )
        .arg(
            Arg::with_name("config_right")
                .long("config_right")
                .value_name("FILE|JSON")
                .number_of_values(1)
                .conflicts_with("config")
                .help("Sets the configuration parameters for the right eye"),
        )
        .arg(
            Arg::with_name("config_left")
                .long("config_left")
                .value_name("FILE|JSON")
                .number_of_values(1)
                .conflicts_with("config")
                .help("Sets the configuration parameters for the left eye"),
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
            Arg::with_name("res")
                .long("res")
                .value_names(&["X", "Y"])
                .number_of_values(2)
                .help("Sets the internal render resolution"),
        )
        .arg(
            Arg::with_name("show")
                .long("show")
                .short("s")
                .takes_value(false)
                .help("Forces window visibility"),
        )
        .arg(
            Arg::with_name("base_image")
                .long("base_image")
                .number_of_values(1)
                .help("The type of base imageto use"),
        )
        .arg(
            Arg::with_name("mix_type")
                .long("mix_type")
                .number_of_values(1)
                .help("The type mix function to use"),
        )
        .arg(
            Arg::with_name("cm_type")
                .long("cm_type")
                .number_of_values(1)
                .help("The type of color map to use"),
        )
        .arg(
            Arg::with_name("cf")
                .long("cf")
                .number_of_values(1)
                .help("The type of combination function to use"),
        )
        .arg(
            Arg::with_name("cm_scale")
                .long("cm_scale")
                .number_of_values(1)
                .help("The factor to scale the colormap by before output"),
        )
        .arg(
            Arg::with_name("perf")
                .long("perf")
                .number_of_values(1)
                .help("Tracks performance metrics"),
        )
        .arg(
            Arg::with_name("variance")
                .long("variance")
                .value_names(&["Type", "Metric", "Color"])
                .number_of_values(3)
                .help("Enables variance metrics. Needs three values:\n\
                \x20\x20Type(0-3): Off/Before/After/Diff\n\
                \x20\x20Metric(0-6): First/Avg/Var_Avg/First_Contrast/Delta_E/Michelson_Contrast/Histogram\n\
                \x20\x20Color(0-2): RGB/LAB/ITP"),
        )
        .arg(
            Arg::with_name("rays")
                .long("rays")
                .number_of_values(1)
                .help("Set of rays to use"),
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
                    \x20\x20\"{{dirname}}/{{stem}}_out.{{extension}}\"  (default)\n\
                    \x20\x20\"{{dirname}}/out_{{basename}}\"",
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

    let default = Config::default();

    let port = if let Some(port_str) = matches.value_of("port") {
        Some(port_str.parse::<u16>().expect("Invalid port"))
    } else {
        default.port
    };

    let mut parameters = if let Some(config_str) = matches.value_of("config") {
        from_json_obj(config_str).expect("Invalid JSON")
    } else {
        default.parameters
    };

    let mut parameters_r = if let Some(config_str) = matches.value_of("config_right") {
        Some(from_json_obj(config_str).expect("Invalid JSON"))
    } else {
        None
    };
    let mut parameters_l = if let Some(config_str) = matches.value_of("config_left") {
        Some(from_json_obj(config_str).expect("Invalid JSON"))
    } else {
        None
    };

 

    if let Some(base_image) = matches.value_of("base_image") {
        let base_image = base_image.parse::<u16>().expect("Invalid base_image") as f64;
        parameters.insert("file_base_image".to_string(), Value::Number(base_image));
    }
    if let Some(mix_type) = matches.value_of("mix_type") {
        let mix_type = mix_type.parse::<u16>().expect("Invalid mix_type") as f64;
        parameters.insert("file_mix_type".to_string(), Value::Number(mix_type));
    }
    if let Some(cm_type) = matches.value_of("cm_type") {
        let cm_type = cm_type.parse::<u16>().expect("Invalid cm_type") as f64;
        parameters.insert("file_cm_type".to_string(), Value::Number(cm_type));
    }
    if let Some(cf) = matches.value_of("cf") {
        let cf = cf.parse::<u16>().expect("Invalid cf") as f64;
        parameters.insert("file_cf".to_string(), Value::Number(cf));
    }

    let mut track_perf = matches.value_of("perf").map(|v| v.parse::<u32>()).unwrap_or(Ok(0u32)).unwrap_or(0);

    if let Some(variance) = matches.values_of("variance") {
        let variance = variance
            .map(|t| t.trim().parse::<f64>().unwrap())
            .collect::<Vec<f64>>();
        parameters.insert("measure_variance".to_string(), Value::Number(variance[0]));
        parameters.insert("variance_metric".to_string(), Value::Number(variance[1]));
        parameters.insert("variance_color_space".to_string(), Value::Number(variance[2]));
    };

    if let Some(rays) = matches.value_of("rays") {
        let rays = rays.parse::<f64>().expect("Invalid rays");
        parameters.insert("rays".to_string(), Value::Number(rays));
    }

    if let Some(cm_scale) = matches.value_of("cm_scale") {
        let cm_scale = cm_scale.parse::<f64>().expect("Invalid cm_scale");
        parameters.insert("cm_scale".to_string(), Value::Number(cm_scale));
    }
    if let Some(vis_type) = matches.value_of("vis_type") {
        let vis_type = vis_type.parse::<u16>().expect("Invalid vis_type") as f64;
        parameters.insert("file_vis_type".to_string(), Value::Number(vis_type));
        // if let Some(parameters_r) = &mut parameters_r{
        //     parameters_r.insert("file_vis_type".to_string(), Value::Number(vis_type));
        // }
        // if let Some(parameters_l) = &mut parameters_l{
        //     parameters_l.insert("file_vis_type".to_string(), Value::Number(vis_type));
        // }
    } 

    if let Some(gaze) = matches.values_of("gaze") {
        let gaze = gaze
            .map(|t| t.trim().parse::<f64>().unwrap())
            .collect::<Vec<f64>>();
        parameters.insert("gaze_x".to_string(), Value::Number(gaze[0]));
        parameters.insert("gaze_y".to_string(), Value::Number(gaze[1]));
    };

    if let Some(view) = matches.values_of("view") {
        let view = view
            .map(|t| t.trim().parse::<f64>().unwrap())
            .collect::<Vec<f64>>();
        parameters.insert("view_x".to_string(), Value::Number(view[0]));
        parameters.insert("view_y".to_string(), Value::Number(view[1]));
    };
    
    let (merged_parameters_r,merged_parameters_l) = match (parameters_r.clone(), parameters_l.clone()){
        (Some(mut parameters_r),Some(mut parameters_l)) =>{
            for (key, val) in parameters.iter() {
                parameters_l.insert(key.clone(),val.clone());
                parameters_r.insert(key.clone(),val.clone());
            }
            (Some(parameters_r),Some(parameters_l))
        }
        _ => (None,None)
    };
    // this is ugly as hell but rust currently does not allow destructuring assignments
    parameters_r = merged_parameters_r;
    parameters_l = merged_parameters_l;


    let mut resolution = default.resolution;
    if let Some(res) = matches.values_of("res") {
        let res = res
            .map(|t| t.trim().parse::<f64>().unwrap())
            .collect::<Vec<f64>>();
        resolution = Some([res[0] as u32, res[1] as u32]);
    };

    let mut visible = default.visible;
    if matches.is_present("show") {
        visible = true;
    }

    let output = if matches.is_present("output") {
        let output = if let Some(output) = matches.value_of("output") {
            output
        } else {
            "{{dirname}}/{{stem}}_out.{{extension}}"
        };
        Some(mustache::compile_str(output).unwrap())
    } else {
        visible = true;
        default.output
    };

    let inputs = matches
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

    Config {
        port,
        visible,
        resolution,
        output,
        inputs,
        parameters,
        parameters_r,
        parameters_l,
        track_perf
    }
}
