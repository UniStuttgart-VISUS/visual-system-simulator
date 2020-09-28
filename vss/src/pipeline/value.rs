use std::collections::HashMap;

#[derive(Debug)]
pub enum Value {
    Bool(bool),
    Number(f64),
    Image(String),
}

impl Value {
    pub fn as_bool(&self) -> Option<bool> {
        match *self {
            Value::Bool(b) => Some(b),
            _ => None,
        }
    }

    pub fn as_f64(&self) -> Option<f64> {
        match *self {
            Value::Number(f) => Some(f),
            _ => None,
        }
    }

    pub fn as_img(&self) -> Option<&str> {
        match *self {
            Value::Image(ref s) => Some(s),
            _ => None,
        }
    }
}

pub type ValueMap = HashMap<String, Value>;
