mod colorblindness;
mod glaucoma;
mod macular_degeneration;
mod nyctalopia;
mod osterberg;
mod receptor_density;

use cgmath::Vector3;

use crate::*;

pub struct RetinaMapBuilder {
    glaucoma_onoff: bool,
    glaucoma_fov: i32,

    achromatopsia_onoff: bool,
    achromatopsia_int: i32,

    nyctalopia_onoff: bool,
    nyctalopia_int: i32,

    colorblindness_onoff: bool,
    colorblindness_type: i32,
    colorblindness_int: i32,

    maculardegeneration_onoff: bool,
    maculardegeneration_veasy: bool,
    maculardegeneration_inteasy: i32,
    maculardegeneration_vadvanced: bool,
    maculardegeneration_radius: f64,
    maculardegeneration_intadvanced: f64,

    receptordensity_onoff: bool,
}

impl RetinaMapBuilder {
    pub fn new() -> Self {
        RetinaMapBuilder {
            glaucoma_onoff: false,
            glaucoma_fov: 0,
            achromatopsia_onoff: false,
            achromatopsia_int: 0,
            nyctalopia_onoff: false,
            nyctalopia_int: 0,
            colorblindness_onoff: false,
            colorblindness_type: 0,
            colorblindness_int: 0,
            maculardegeneration_onoff: false,
            maculardegeneration_veasy: false,
            maculardegeneration_inteasy: 0,
            maculardegeneration_vadvanced: false,
            maculardegeneration_radius: 0.0,
            maculardegeneration_intadvanced: 0.0,
            receptordensity_onoff: false,
        }
    }

    pub fn inspect(&mut self, inspector: &mut dyn Inspector) {
        inspector.mut_bool("glaucoma_onoff", &mut self.glaucoma_onoff);
        inspector.mut_i32("glaucoma_fov", &mut self.glaucoma_fov);

        inspector.mut_bool("achromatopsia_onoff", &mut self.achromatopsia_onoff);
        inspector.mut_i32("achromatopsia_int", &mut self.achromatopsia_int);

        inspector.mut_bool("nyctalopia_onoff", &mut self.nyctalopia_onoff);
        inspector.mut_i32("nyctalopia_int", &mut self.nyctalopia_int);

        inspector.mut_bool("colorblindness_onoff", &mut self.colorblindness_onoff);
        inspector.mut_i32("colorblindness_type", &mut self.colorblindness_type);
        inspector.mut_i32("colorblindness_int", &mut self.colorblindness_int);

        inspector.mut_bool(
            "maculardegeneration_onoff",
            &mut self.maculardegeneration_onoff,
        );
        inspector.mut_bool(
            "maculardegeneration_veasy",
            &mut self.maculardegeneration_veasy,
        );
        inspector.mut_i32(
            "maculardegeneration_inteasy",
            &mut self.maculardegeneration_inteasy,
        );
        inspector.mut_bool(
            "maculardegeneration_vadvanced",
            &mut self.maculardegeneration_vadvanced,
        );
        inspector.mut_f64(
            "maculardegeneration_radius",
            &mut self.maculardegeneration_radius,
        );
        inspector.mut_f64(
            "maculardegeneration_intadvanced",
            &mut self.maculardegeneration_intadvanced,
        );

        inspector.mut_bool("receptordensity_onoff", &mut self.receptordensity_onoff);
    }
    
    pub fn generate(&self, resolution: (u32, u32), orientation: &[Vector3<f32>; 3]) -> Box<[u8]> {
        let mut maps: Vec<image::ImageBuffer<image::Rgba<u8>, Vec<u8>>> = Vec::new();

        // glaucoma
        if self.glaucoma_onoff {
            let severity = self.glaucoma_fov as u8;
            //let glaucoma_scotomasize = values.get("glaucoma_scotomasize"].as_u64().unwrap();
            let glaucoma = glaucoma::generate_simple(resolution, orientation, severity);
            maps.push(glaucoma);
        }

        // achromatopsia
        if self.achromatopsia_onoff {
            let severity = self.achromatopsia_int as u8;
            let achromatopsia = colorblindness::generate_achromatopsia(resolution, severity);
            maps.push(achromatopsia);
        }

        // nyctalopia
        if self.nyctalopia_onoff {
            let severity = self.nyctalopia_int as u8;
            let nyctalopia = nyctalopia::generate(resolution, severity);
            maps.push(nyctalopia);
        }

        // colorblindness
        if self.colorblindness_onoff {
            let ctype = self.colorblindness_type as u8;
            let severity = self.colorblindness_int as u8;
            let colorblindness =
                colorblindness::generate_colorblindness(resolution, ctype, severity);
            maps.push(colorblindness);
        }

        // macular degeneration
        if self.maculardegeneration_onoff {
            if self.maculardegeneration_veasy {
                // parameters set in easy easy mode
                let severity = self.maculardegeneration_inteasy as u8;
                let macular_degeneration =
                    macular_degeneration::generate_simple(resolution, orientation, severity);
                maps.push(macular_degeneration);
            } else if self.maculardegeneration_vadvanced {
                // parameters set in advanced mode
                let radius = self.maculardegeneration_radius;
                let severity = self.maculardegeneration_intadvanced;
                // interpret parameters
                let severity = 1.0 - 0.5 * (1.0 - severity / 100.0).powi(2);
                let macular_degeneration = macular_degeneration::generate(
                    resolution,
                    orientation,
                    radius / 100.0,
                    severity,
                );
                maps.push(macular_degeneration);
            }
        }

        // receptor density
        if self.receptordensity_onoff {
            let receptor_density = receptor_density::generate(resolution);
            maps.push(receptor_density);
        }

        Self::merge_maps(maps, resolution)
    }

    fn merge_maps(
        maps: Vec<image::ImageBuffer<image::Rgba<u8>, Vec<u8>>>,
        resolution: (u32, u32),
    ) -> Box<[u8]> {
        // generate white retina map as starting point
        let mut merged = image::ImageBuffer::new(resolution.0, resolution.1);
        for (_, _, pixel) in merged.enumerate_pixels_mut() {
            *pixel = image::Rgba([255 as u8, 255 as u8, 255 as u8, 255 as u8]);
        }

        // for each pixel and each channel, take the minimum of all maps at this pixel and channel
        for map in maps {
            for (x, y, pixel) in merged.enumerate_pixels_mut() {
                let new_pixel = map.get_pixel(x, y);
                let r = new_pixel[0].min(pixel[0]);
                let g = new_pixel[1].min(pixel[1]);
                let b = new_pixel[2].min(pixel[2]);
                let a = new_pixel[3].min(pixel[3]);
                *pixel = image::Rgba([r as u8, g as u8, b as u8, a as u8]);
            }
        }

        merged.into_raw().into_boxed_slice()
    }
}
