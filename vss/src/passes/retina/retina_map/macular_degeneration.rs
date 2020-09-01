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
    let severity: f64 = severity as f64 / 100.0;
    let severity: f64 = 1.0 - 0.5 * (1.0 - severity).powi(2);
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
/// - `factor` - intensity of the effect, value between 0 and 1
///
pub fn generate(
    res: (u32, u32),
    radius: f64,
    factor: f64,
) -> image::ImageBuffer<image::Rgba<u8>, Vec<u8>> {
    let mut mapbuffer = image::ImageBuffer::new(res.0, res.1);

    for (x, y, pixel) in mapbuffer.enumerate_pixels_mut() {
        let distance_from_center = ((((res.0 / 2) as i32 - x as i32).pow(2)
            + ((res.1 / 2) as i32 - y as i32).pow(2)) as f64)
            .sqrt();

        let x = 1.0 - (radius - distance_from_center).max(0.0) / radius;
        //                        * 1.78 enlarges the black spot in the center
        //                                                                   - 0.72 makes sure the outer areas are 0
        let cells = 255
            - (255.0 * (1.78 * factor * (-1.0 * x.powi(2)).exp() - 0.72))
                .max(0.0)
                .min(255.0) as u8;

        *pixel = image::Rgba([cells, cells, cells, cells]);
    }

    mapbuffer
}
