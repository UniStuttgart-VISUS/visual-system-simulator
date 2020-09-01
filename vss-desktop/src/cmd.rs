use serde_json;

use std::fs::File;
use std::io::prelude::*;
use vss::*;

fn from_json(arg: &str) -> Result<serde_json::Value, serde_json::Error> {
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

fn from_json_obj(arg: &str) -> Result<ValueMap, serde_json::Error> {
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
    use clap::{App, Arg};
    let version = "1.0.0";
    let title = "Visual System Simulator (VSS)";
    let description = "Simulates various aspects of the human visual system";
    let authors = "The Visual System Simulator Developers";

    let matches = App::new(title)
        .version(version)
        .author(authors)
        .about(description)
        .arg(
            Arg::with_name("port")
                .long("port")
                .short("p")
                .value_name("PORT")
                .help("Specifies the port on which the server listens for connections")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("config")
                .long("config")
                .short("c")
                .value_name("FILE|JSON")
                .help("Sets the configuration parameters for simulation"),
        )
        .arg(
            Arg::with_name("gaze")
                .long("gaze")
                .short("g")
                .value_name("X,Y")
                .help("Sets the fallback gaze position"),
        )
        .arg(
            Arg::with_name("device")
                .value_name("DEVICE")
                .possible_values(&["image", "video", "camera"])
                .help("Sets the device type")
                .required(true)
                .index(1),
        )
        .arg(
            Arg::with_name("input")
                .value_name("SOURCE")
                .help("Sets a device-dependent data source, e.g., a file or device identifier")
                .required(true)
                .index(2),
        )
        .arg(
            Arg::with_name("output")
                .value_name("SINK")
                .help("Sets a device-dependent data sink, e.g., a file or device identifier")
                .index(3),
        )
        .get_matches();

    let default = Config::default();

    let port = if let Some(port_str) = matches.value_of("port") {
        port_str.parse::<u16>().expect("Invalid port")
    } else {
        default.port
    };

    let gaze = if let Some(gaze_str) = matches.value_of("gaze") {
        let numbers = gaze_str
            .split(",")
            .map(|t| t.trim().parse::<u32>().unwrap())
            .collect::<Vec<u32>>();
        Some(DeviceGaze {
            x: numbers[0] as f32,
            y: numbers[1] as f32,
        })
    } else {
        default.gaze
    };

    let parameters = if let Some(config_str) = matches.value_of("config") {
        from_json_obj(config_str).expect("Invalid JSON")
    } else {
        default.parameters
    };

    let device = matches
        .value_of("device")
        .unwrap_or(default.device.as_str())
        .to_string();
    let input = matches
        .value_of("input")
        .unwrap_or(default.input.as_str())
        .to_string();
    let output = matches
        .value_of("output")
        .unwrap_or(default.output.as_str())
        .to_string();

    Config {
        port,
        device,
        input,
        output,
        gaze,
        parameters,
    }
}
