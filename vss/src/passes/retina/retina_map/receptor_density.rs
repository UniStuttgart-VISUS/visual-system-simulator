use image;

use super::osterberg::*;

/// Gaussian apodization function
fn gaussian(x: f32, sigma: f32) -> f32 {
    (-x.powi(2) / (2.0 * sigma.powi(2))).exp()
}

/// Generates a retina map that can be used to simulate receptor density, i.e., foveal vs. peripheral vision.
pub fn generate(screen_size_px: (u32, u32)) -> image::ImageBuffer<image::Rgba<u8>, Vec<u8>> {
    let mut map = image::ImageBuffer::new(screen_size_px.0, screen_size_px.1);

    for (x, y, pixel) in map.enumerate_pixels_mut() {
        let t = (x as f32) / (screen_size_px.0 as f32);
        let u = (y as f32) / (screen_size_px.1 as f32);

        let eccentricity = t * 220.0 - 110.0;
        let (cone_density, rod_density) = osterberg(eccentricity);
        let cone_density = cone_density / CONE_DENSITY_MAX;
        let rod_density = rod_density / ROD_DENSITY_MAX;

        //XXX: this is crude, but okay for now.
        let alpha = gaussian(u * 2.0 - 1.0, 0.125);
        let base_density = (gaussian(t * 2.0 - 1.0, 0.35) - 0.14).max(0.0)
            * (gaussian(u * 2.0 - 1.0, 0.5) - 0.14).max(0.0);
        let c = (alpha * cone_density + (1.0 - alpha) * base_density) * 255.0;
        let r = (alpha * rod_density + (1.0 - alpha) * base_density) * 255.0;

        *pixel = image::Rgba([c as u8, c as u8, c as u8, r as u8]);
    }

    map
}
