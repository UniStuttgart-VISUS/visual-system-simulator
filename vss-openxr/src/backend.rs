use std::fmt;

/// Runtime-selected OpenXR graphics backend.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Backend {
    Auto,
    Vulkan,
    Metal,
}

impl Backend {
    pub fn parse(value: &str) -> Result<Self, BackendParseError> {
        match value {
            "auto" => Ok(Self::Auto),
            "vulkan" => Ok(Self::Vulkan),
            "metal" => Ok(Self::Metal),
            other => Err(BackendParseError {
                value: other.to_string(),
            }),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BackendParseError {
    value: String,
}

impl fmt::Display for BackendParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "unknown OpenXR backend '{}'; expected auto, vulkan, or metal",
            self.value
        )
    }
}

impl std::error::Error for BackendParseError {}
