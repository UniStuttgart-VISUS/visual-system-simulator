///
/// Creates a retina map that can be used to simulate colorblindness.
///
/// # Arguments
///
/// - `res`      - resolution of the returned retina map
/// - `type`     - type of colorblindness (red, green, blue)
/// - `severity` - the severity of the disease, value between 0 and 100
///
pub fn generate_colorblindness(
    res: (u32, u32),
    ctype: u8,
    severity: u8,
) -> image::ImageBuffer<image::Rgba<u8>, Vec<u8>> {
    let mut mapbuffer = image::ImageBuffer::new(res.0, res.1);

    for (_, _, pixel) in mapbuffer.enumerate_pixels_mut() {
        let mut r = 255;
        let mut g = 255;
        let mut b = 255;

        if ctype == 0 {
            r = 255 - (255 * severity) / 100;
        } else if ctype == 1 {
            g = 255 - (255 * severity) / 100;
        } else if ctype == 2 {
            b = 255 - (255 * severity) / 100;
        }

        *pixel = image::Rgba([r as u8, g as u8, b as u8, 255]);
    }

    mapbuffer
}

///
/// Creates a retina map that can be used to simulate achromatopsia.
///
/// # Arguments
///
/// - `res`      - resolution of the returned retina map
/// - `severity` - the severity of the disease, value between 0 and 100
///
pub fn generate_achromatopsia(
    res: (u32, u32),
    severity: u8,
) -> image::ImageBuffer<image::Rgba<u8>, Vec<u8>> {
    let mut mapbuffer = image::ImageBuffer::new(res.0, res.1);

    for (_, _, pixel) in mapbuffer.enumerate_pixels_mut() {
        let v = 255 - (255 * severity) / 100;
        *pixel = image::Rgba([v as u8, v as u8, v as u8, 255]);
    }

    mapbuffer
}
