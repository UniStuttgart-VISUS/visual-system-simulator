use crate::*;

use std::cell::RefCell;

use cgmath::Matrix4;
use egui::TextBuffer;

pub trait Inspector {
    fn flow(&mut self, index: usize, flow: &Flow);

    fn mut_node(&mut self, node: &mut dyn Node);

    // Returns true if value was changed.
    fn mut_bool(&mut self, name: &'static str, value: &mut bool) -> bool;
    fn mut_f64(&mut self, name: &'static str, value: &mut f64) -> bool;
    fn mut_f32(&mut self, name: &'static str, value: &mut f32) -> bool;
    fn mut_i32(&mut self, name: &'static str, value: &mut i32) -> bool;
    fn mut_u32(&mut self, name: &'static str, value: &mut u32) -> bool;
    fn mut_img(&mut self, name: &'static str, value: &mut String) -> bool;
    fn mut_matrix(&mut self, name: &'static str, value: &mut Matrix4<f32>) -> bool;
}

#[derive(Debug)]
pub enum JsonError {
    Serde(serde_json::error::Error),
    ExpectedFlowArray,
}

impl From<serde_json::error::Error> for JsonError {
    fn from(err: serde_json::error::Error) -> JsonError {
        JsonError::Serde(err)
    }
}

pub struct FromJsonInspector<'a> {
    flows: Vec<serde_json::value::Value>,
    current_flow: Option<&'a serde_json::Map<String, serde_json::Value>>,
    current_node: Option<&'a serde_json::Map<String, serde_json::Value>>,
}

impl<'a> FromJsonInspector<'a> {
    pub fn try_new(json_string: &str) -> Result<Self, JsonError> {
        let json = serde_json::from_str(json_string)?;
        if let serde_json::Value::Array(flows) = json {
            Ok(Self {
                flows,
                current_flow: None,
                current_node: None,
            })
        } else {
            Err(JsonError::ExpectedFlowArray)
        }
    }

    fn node_attribute(&mut self, name: &'static str) -> Option<&'a serde_json::Value> {
        self.current_node.and_then(|node| node.get(name))
    }
}

impl<'a> Inspector for FromJsonInspector<'a> {
    fn flow(&mut self, index: usize, flow: &Flow) {
        //TODO: self.current_flow = self.flows.get(index).and_then(|flow| flow.as_object());
        flow.inspect(self);
        self.current_flow = None;
    }

    fn mut_node(&mut self, node: &mut dyn Node) {
        self.current_node = self
            .current_flow
            .and_then(|flow| flow.get(node.name()))
            .and_then(|node| node.as_object());

        node.inspect(self);

        self.current_node = None;
    }

    fn mut_bool(&mut self, name: &'static str, value: &mut bool) -> bool {
        if let Some(serde_json::value::Value::Bool(json_value)) = self.node_attribute(name) {
            *value = *json_value;
            true
        } else {
            false
        }
    }

    fn mut_f64(&mut self, name: &'static str, value: &mut f64) -> bool {
        if let Some(serde_json::value::Value::Number(json_value)) = self.node_attribute(name) {
            *value = json_value.as_f64().unwrap();
            true
        } else {
            false
        }
    }

    fn mut_f32(&mut self, name: &'static str, value: &mut f32) -> bool {
        if let Some(serde_json::value::Value::Number(json_value)) = self.node_attribute(name) {
            *value = json_value.as_f64().unwrap() as f32;
            true
        } else {
            false
        }
    }

    fn mut_i32(&mut self, name: &'static str, value: &mut i32) -> bool {
        if let Some(serde_json::value::Value::Number(json_value)) = self.node_attribute(name) {
            *value = json_value.as_f64().unwrap() as i32;
            true
        } else {
            false
        }
    }

    fn mut_u32(&mut self, name: &'static str, value: &mut u32) -> bool {
        if let Some(serde_json::value::Value::Number(json_value)) = self.node_attribute(name) {
            *value = json_value.as_f64().unwrap() as u32;
            true
        } else {
            false
        }
    }

    fn mut_img(&mut self, name: &'static str, value: &mut String) -> bool {
        if let Some(serde_json::value::Value::String(json_value)) = self.node_attribute(name) {
            *value = json_value.to_string();
            true
        } else {
            false
        }
    }

    fn mut_matrix(&mut self, _name: &'static str, _value: &mut cgmath::Matrix4<f32>) -> bool {
        false //TODO: implement this as needed.
    }
}

pub struct ToJsonInspector {
    flows: Vec<serde_json::Value>,
    current_flow: RefCell<serde_json::Map<String, serde_json::Value>>,
    current_node_attributes: RefCell<serde_json::Map<String, serde_json::Value>>,
    current_node_name: String,
}

impl ToJsonInspector {
    pub fn new() -> Self {
        Self {
            flows: Vec::new(),
            current_flow: RefCell::new(serde_json::Map::new()),
            current_node_attributes: RefCell::new(serde_json::Map::new()),
            current_node_name: String::new(),
        }
    }

    pub fn to_string(self) -> String {
        serde_json::Value::Array(self.flows).to_string()
    }

    fn insert_attribute(&mut self, name: &'static str, value: serde_json::Value) -> bool {
        self.current_node_attributes
            .borrow_mut()
            .insert(name.to_string(), value);
        false
    }
}

impl Inspector for ToJsonInspector {
    fn flow(&mut self, index: usize, flow: &Flow) {
        assert_eq!(
            index,
            self.flows.len(),
            "Indices must be accessed in ascending consecutive order"
        );
        self.current_flow.borrow_mut().clear();

        flow.inspect(self);

        self.flows
            .push(serde_json::Value::Object(self.current_flow.take()));
    }

    fn mut_node(&mut self, node: &mut dyn Node) {
        self.current_node_attributes.borrow_mut().clear();
        self.current_node_name = node.name().to_string();

        node.inspect(self);

        self.current_flow.borrow_mut().insert(
            self.current_node_name.take(),
            serde_json::Value::Object(self.current_node_attributes.take()),
        );
    }

    fn mut_bool(&mut self, name: &'static str, value: &mut bool) -> bool {
        self.insert_attribute(name, serde_json::Value::Bool(*value))
    }

    fn mut_f64(&mut self, name: &'static str, value: &mut f64) -> bool {
        self.insert_attribute(name, serde_json::Value::from(*value as f64))
    }

    fn mut_f32(&mut self, name: &'static str, value: &mut f32) -> bool {
        self.insert_attribute(name, serde_json::Value::from(*value as f64))
    }

    fn mut_i32(&mut self, name: &'static str, value: &mut i32) -> bool {
        self.insert_attribute(name, serde_json::Value::from(*value as f64))
    }

    fn mut_u32(&mut self, name: &'static str, value: &mut u32) -> bool {
        self.insert_attribute(name, serde_json::Value::from(*value as f64))
    }

    fn mut_img(&mut self, name: &'static str, value: &mut String) -> bool {
        self.insert_attribute(name, serde_json::Value::String(value.clone()))
    }

    fn mut_matrix(&mut self, _name: &'static str, _value: &mut cgmath::Matrix4<f32>) -> bool {
        false //TODO: implement this as needed.
    }
}
