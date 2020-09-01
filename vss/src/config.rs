use crate::devices::*;
use crate::pipeline::*;

pub use std::collections::HashMap;

#[derive(Debug)]
pub struct Config {
    pub port: u16,
    pub device: String,
    pub input: String,
    pub output: String,
    pub gaze: Option<DeviceGaze>,
    pub parameters: ValueMap,
}

#[derive(Debug, Clone)]
pub enum ConfigError {
    UnknownDevice,
}

impl Config {
    pub fn build<F>(self, resolve_device_fn: F) -> Result<(Box<dyn Device>, Pipeline), ConfigError>
    where
        F: Fn(&Config) -> Option<Box<dyn Device>>,
    {
        // Resolve device name to instance.
        let mut device = if let Some(device) = resolve_device_fn(&self) {
            Ok(device)
        } else {
            match self.device.as_ref() {
                "image" | "still" => Ok(Box::new(ImageDevice::new(&self)) as Box<dyn Device>),
                _ => Err(ConfigError::UnknownDevice),
            }
        }?;

        // Wrap in a remote device, if requested.
        device = if self.port != 0 {
            Box::new(RemoteDevice::new(&self, device))
        } else {
            device
        };

        let pipeline = Pipeline::new(&mut device, self.parameters);
        Ok((device, pipeline))
    }
}

impl Default for Config {
    fn default() -> Self {
        Config {
            port: 9009,
            device: "image".to_string(),
            input: "".to_string(),
            output: "".to_string(),
            gaze: None,
            parameters: HashMap::new(),
        }
    }
}
