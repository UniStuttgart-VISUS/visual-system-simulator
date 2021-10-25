///
/// Creates a retina map that can be used to simulate nyctalopia.
///
/// # Arguments
///
/// - `res`      - resolution of the returned retina map
/// - `severity` - the severity of the disease, value between 0 and 100
///
pub fn generate(res: (u32, u32), severity: u8) -> image::ImageBuffer<image::Rgba<u8>, Vec<u8>> {
    let mut mapbuffer = image::ImageBuffer::new(res.0, res.1);

    for (_, _, pixel) in mapbuffer.enumerate_pixels_mut() {
        let a = 255 - (255 * severity as u32) / 100;

        *pixel = image::Rgba([255, 255, 255, a as u8]);
    }

    mapbuffer
}
