use itertools::Itertools;
use std::error::Error;
use std::fs::File;
use std::io::prelude::*;
use vss::*;

#[derive(Debug)]
pub struct Config {
    pub port: u16,
    pub ios: Vec<(String, String)>,
    pub parameters: ValueMap,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            port: 9009,
            ios: Vec::new(),
            parameters: std::collections::HashMap::new(),
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
                .help("Specifies the port on which the server listens for connections")
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
            Arg::with_name("gaze")
                .long("gaze")
                .short("g")
                .value_names(&["X", "Y"])
                .number_of_values(2)
                .help("Sets the fallback gaze position"),
        )
        .arg(
            Arg::with_name("INPUT_OUTPUT")
                .value_names(&["INPUT", "OUTPUT"])
                .help(
                    "Input and output identifiers in pairs of two, e.g.:\n\
                    \x20\x20input1.png\x20\x20(automatically chosen output)\n\
                    \x20\x20input1.png output1.png\n\
                    \x20\x20input1.png output1.png input2.png output2.png ...",
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
        port_str.parse::<u16>().expect("Invalid port")
    } else {
        default.port
    };

    let mut parameters = if let Some(config_str) = matches.value_of("config") {
        from_json_obj(config_str).expect("Invalid JSON")
    } else {
        default.parameters
    };

    if let Some(gaze) = matches.values_of("gaze") {
        let gaze = gaze
            .map(|t| t.trim().parse::<f64>().unwrap())
            .collect::<Vec<f64>>();
        parameters.insert("gaze_x".to_string(), Value::Number(gaze[0]));
        parameters.insert("gaze_y".to_string(), Value::Number(gaze[1]));
    };

    let mut io_tuples = matches
        .values_of("INPUT_OUTPUT")
        .unwrap()
        .map(|id| id.to_string())
        .tuples();
    let mut ios = Vec::new();
    for (input, output) in io_tuples.by_ref() {
        ios.push((input, output));
    }
    if ios.is_empty() {
    for input in io_tuples.into_buffer() {
        ios.push((input, String::new()));
    }
}

    Config {
        port,
        ios,
        parameters,
    }
}
