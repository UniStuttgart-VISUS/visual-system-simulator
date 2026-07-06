use std::fmt::{Display, Formatter};

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct AssetId {
    raw: String,
}

impl AssetId {
    pub fn from_str(raw: impl Into<String>) -> Self {
        Self { raw: raw.into() }
    }

    pub fn new() -> Self {
        Self { raw: "".into() }
    }

    pub fn is_empty(&self) -> bool {
        self.raw.is_empty()
    }

    pub fn raw(&self) -> &str {
        &self.raw
    }
}

impl From<String> for AssetId {
    fn from(raw: String) -> Self {
        Self::from_str(raw)
    }
}

impl From<&str> for AssetId {
    fn from(raw: &str) -> Self {
        Self::from_str(raw)
    }
}

impl Display for AssetId {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.raw)
    }
}
