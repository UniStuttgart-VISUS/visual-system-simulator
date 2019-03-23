use image;

///
/// Creates a retina map that can be used to simulate glaucoma.
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
    if severity > 98 {
        generate_blindness(res)
    } else {
        let radius = ((((res.0 / 2) as i32).pow(2) + ((res.1 / 2) as i32).pow(2)) as f64).sqrt();
        generate(res, radius, severity)
    }
}

///
/// Creates a retina map that can be used to simulate glaucoma.
///
/// # Arguments
///
/// - `res`    - resolution of the returned retina map
/// - `radius` - radius of the affected area
/// - `factor` - intensity of the effect, value between 0 and 1
///
fn generate(
    res: (u32, u32),
    radius: f64,
    severity: u8,
) -> image::ImageBuffer<image::Rgba<u8>, Vec<u8>> {
    let severity = severity as f64;

    let border_width = if severity >= 80.0 {
        10.0 * (100.0 - severity)
    } else if severity < 20.0 {
        10.0 * severity
    } else {
        200.0
    };

    let mut mapbuffer = image::ImageBuffer::new(res.0, res.1);

    for (x, y, pixel) in mapbuffer.enumerate_pixels_mut() {
        let distance_from_center = ((((res.0 / 2) as i32 - x as i32).pow(2)
            + ((res.1 / 2) as i32 - y as i32).pow(2)) as f64)
            .sqrt();

        let factor = 1.0 - (severity / 100.0);

        let mut is_healthy = true;
        if distance_from_center > radius * factor {
            is_healthy = false;
        }

        let mut cells = 255;
        if !is_healthy {
            cells = 0;

            // partly affected cells just outside the healthy circle
            cells += (255.0 / border_width) as u8
                * (border_width - (distance_from_center - radius * factor))
                    .min(border_width)
                    .max(0.0) as u8;
        }

        let cells = calculate_scotoma(res, (x, y), severity, cells);

        *pixel = image::Rgba([cells, cells, cells, cells]);
    }

    mapbuffer
}

#[allow(unused_variables)]
fn calculate_scotoma(res: (u32, u32), pos: (u32, u32), severity: f64, cells: u8) -> u8 {
    cells
}

///
/// Creates a retina map that can be used to simulate absolute blindness.
///
/// # Arguments
///
/// - `res`    - resolution of the returned retina map
///
pub fn generate_blindness(res: (u32, u32)) -> image::ImageBuffer<image::Rgba<u8>, Vec<u8>> {
    let mut mapbuffer = image::ImageBuffer::new(res.0, res.1);

    for (_, _, pixel) in mapbuffer.enumerate_pixels_mut() {
        *pixel = image::Rgba([0, 0, 0, 0]);
    }

    mapbuffer
}
