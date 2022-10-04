// mod download;
mod upload;

// pub use download::*;
pub use upload::*;

use super::*;

// A buffer representing color information.
pub struct RgbBuffer {
    pub pixels_rgb: Box<[u8]>,
    pub width: u32,
    pub height: u32,
}

impl Default for RgbBuffer {
    fn default() -> Self {
        Self {
            pixels_rgb: vec![0; 1].into_boxed_slice(),
            width: 1,
            height: 1,
        }
    }
}
