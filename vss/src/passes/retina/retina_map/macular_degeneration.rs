///
/// Creates a retina map that can be used to simulate macular degeneration.
///
/// # Arguments
///
/// - `res`      - resolution of the returned retina map
/// - `severity` - the severity of the disease, value between 0 and 100
///
pub fn generate_simple(
    res: (u32, u32),
    severity: u8,
) -> image::ImageBuffer<image::Rgba<u8>, Vec<u8>> {
    // convert severity from int between 0 and 100 to float between 0.5 and 1
    let severity = severity as f64 / 100.0;
    let severity = 1.0 - 0.5 * (1.0 - severity).powi(2);
    let radius = severity * res.0 as f64 / 3.0;

    generate(res, radius, severity)
}

///
/// Creates a retina map that can be used to simulate macular degeneration.
///
/// # Arguments
///
/// - `res`    - resolution of the returned retina map
/// - `radius` - radius of the affected area
/// - `intensity` - intensity of the degeneration, value between 0.0 and 1.0
///
pub fn generate(
    res: (u32, u32),
    radius: f64,
    intensity: f64,
) -> image::ImageBuffer<image::Rgba<u8>, Vec<u8>> {
    let mut map = image::ImageBuffer::new(res.0, res.1);

    let cx = (res.0 / 2) as i32;
    let cy = (res.1 / 2) as i32;

    for (x, y, pixel) in map.enumerate_pixels_mut() {
        let distance_squared = (cx - x as i32).pow(2) + (cy - y as i32).pow(2);
        // enlarges the black spot in the center.
        let spot_factor = 1.78;
        // ensures the outer areas are zero.
        let tail_factor = 0.72;
        let relative_falloff =
            1.0 - (radius.powi(2) - distance_squared as f64).max(0.0) / radius.powi(2);
        let x = spot_factor * (-relative_falloff).exp() - tail_factor;
        let cells = 255 - (255.0 * x * intensity).max(0.0).min(255.0) as u8;
        *pixel = image::Rgba([cells, cells, cells, cells]);
    }

    map
}
