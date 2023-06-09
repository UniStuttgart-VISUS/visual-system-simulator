use cgmath::{InnerSpace, Vector3};

///
/// Creates a retina map that can be used to simulate glaucoma.
///
/// # Arguments
///
/// - `res`         - resolution of the returned retina map
/// - `orientation` - right/up/forward vectors
/// - `severity`    - the severity of the disease, value between 0 and 100
///
pub fn generate_simple(
    res: (u32, u32),
    orientation: &[Vector3<f32>; 3],
    severity: u8,
) -> image::ImageBuffer<image::Rgba<u8>, Vec<u8>> {
    if severity > 98 {
        generate_blindness(res)
    } else {
        generate(res, orientation, severity)
    }
}

///
/// Creates a retina map that can be used to simulate glaucoma.
///
/// # Arguments
///
/// - `res`         - resolution of the returned retina map
/// - `orientation` - right/up/forward vectors
/// - `severity`    - intensity of the effect, value between 0 and 1
///
fn generate(
    res: (u32, u32),
    orientation: &[Vector3<f32>; 3],
    severity: u8,
) -> image::ImageBuffer<image::Rgba<u8>, Vec<u8>> {
    let severity = severity as f64 / 100.0;

    let border_width = if severity >= 0.8 {
        0.3 * (1.0 - severity)
    } else if severity < 0.2 {
        0.3 * severity
    } else {
        0.3
    };
    //let border_width = min((1.0 - 2*abs(x-0.5)) * 500, 200);
    let factor = 1.0 - severity;

    let mut mapbuffer = image::ImageBuffer::new(res.0, res.1);

    let global_forward = -Vector3::unit_z(); //Result of OpenGL being Right-handed

    for (x, y, pixel) in mapbuffer.enumerate_pixels_mut() {
        let right = (((x as f32 + 0.5) / res.0 as f32) * 2.0 - 1.0) * orientation[0];
        let up = (((y as f32 + 0.5) / res.1 as f32) * 2.0 - 1.0) * orientation[1];
        let direction = (right + up + orientation[2]).normalize();
        let angle = global_forward.angle(direction).0 as f64;

        let mut cells = 255;
        if angle > factor {
            cells = 0;

            // partly affected cells just outside the healthy circle
            cells += (255.0
                * ((border_width - (angle - factor)) / border_width)
                    .min(1.0)
                    .max(0.0)) as u8;
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
