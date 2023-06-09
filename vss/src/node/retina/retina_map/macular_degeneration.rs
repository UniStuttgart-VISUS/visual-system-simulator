use cgmath::{InnerSpace, Vector3};

const MACULAR_SIZE: f64 = 0.3199770295315; //angular size of macular in radians

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
    orientation: &[Vector3<f32>; 3],
    severity: u8,
) -> image::ImageBuffer<image::Rgba<u8>, Vec<u8>> {
    // convert severity from int between 0 and 100 to float between 0.5 and 1
    let severity = severity as f64 / 100.0;
    let intensity = 1.0 - 0.5 * (1.0 - severity).powi(2);
    let radius = severity.powi(2) * 100.0;

    generate(res, orientation, radius, intensity)
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
    orientation: &[Vector3<f32>; 3],
    radius: f64,
    intensity: f64,
) -> image::ImageBuffer<image::Rgba<u8>, Vec<u8>> {
    let mut map = image::ImageBuffer::new(res.0, res.1);

    let global_forward = -Vector3::unit_z(); //Result of OpenGL being Right-handed
    let affected_angle = radius * MACULAR_SIZE / 2.0;

    for (x, y, pixel) in map.enumerate_pixels_mut() {
        let right = (((x as f32 + 0.5) / res.0 as f32) * 2.0 - 1.0) * orientation[0];
        let up = (((y as f32 + 0.5) / res.1 as f32) * 2.0 - 1.0) * orientation[1];
        let direction = (right + up + orientation[2]).normalize();
        let angle = global_forward.angle(direction).0 as f64;

        // enlarges the black spot in the center.
        let spot_factor = 1.78;
        // ensures the outer areas are zero.
        let tail_factor = 0.72;
        let relative_falloff = 1.0 - (affected_angle - angle).max(0.0) / affected_angle;
        let x = spot_factor * (-relative_falloff).exp() - tail_factor;
        let cells = 255 - (255.0 * x * intensity).max(0.0).min(255.0) as u8;
        *pixel = image::Rgba([cells, cells, cells, cells]);
    }

    map
}
