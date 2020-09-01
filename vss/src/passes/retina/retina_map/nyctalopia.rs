///
/// Creates a retina map that can be used to simulate nyctalopia.
///
/// # Arguments
///
/// - `res`      - resolution of the returned retina map
/// - `severity` - the severity of the disease, value between 0 and 100
///
pub fn generate(res: (u32, u32), severity: u64) -> image::ImageBuffer<image::Rgba<u8>, Vec<u8>> {
    let mut mapbuffer = image::ImageBuffer::new(res.0, res.1);

    for (_, _, pixel) in mapbuffer.enumerate_pixels_mut() {
        let r = 255;
        let g = 255;
        let b = 255;
        let mut a = 255;

        a = a - a * severity / 100;

        *pixel = image::Rgba([r as u8, g as u8, b as u8, a as u8]);
    }

    mapbuffer
}
