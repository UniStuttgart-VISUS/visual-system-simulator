#[cfg(feature = "video")]
mod av_to_rgb;
#[cfg(feature = "video")]
mod rgb_to_av;

#[cfg(feature = "video")]
pub use self::av_to_rgb::*;
#[cfg(feature = "video")]
pub use self::rgb_to_av::*;

#[cfg(not(feature = "video"))]
use vss::unimplemented_node;

#[cfg(not(feature = "video"))]
unimplemented_node!(AvToRgb);

#[cfg(not(feature = "video"))]
unimplemented_node!(RgbToAv);
