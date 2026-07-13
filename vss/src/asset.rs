use std::{
    fmt::{Display, Formatter},
    io::Cursor,
};

#[derive(Debug, Default, Clone, Eq, PartialEq)]
pub struct AssetId(String);

impl AssetId {
    pub fn from_str(raw: impl Into<String>) -> Self {
        Self(raw.into())
    }

    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn raw(&self) -> &str {
        &self.0
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
        f.write_str(&self.0)
    }
}

pub trait AssetLoader: Send + Sync {
    fn load(&self, id: &AssetId) -> Result<Cursor<Vec<u8>>, String>;
}

impl<F> AssetLoader for F
where
    F: Fn(&AssetId) -> Result<Cursor<Vec<u8>>, String> + Send + Sync,
{
    fn load(&self, id: &AssetId) -> Result<Cursor<Vec<u8>>, String> {
        self(id)
    }
}
