mod colorblindness;
mod glaucoma;
mod macular_degeneration;
mod nyctalopia;
mod osterberg;
mod receptor_density;

use image;

use crate::pipeline::*;

pub fn generate_retina_map(resolution: (u32, u32), params: &ValueMap) -> Box<[u8]> {
    let mut maps: Vec<image::ImageBuffer<image::Rgba<u8>, Vec<u8>>> = Vec::new();

    // glaucoma
    if let Some(Value::Bool(true)) = params.get("glaucoma_onoff") {
        let severity = params.get("glaucoma_fov").unwrap().as_f64().unwrap() as u8;
        //let glaucoma_scotomasize = params.get("glaucoma_scotomasize"].as_u64().unwrap();
        let glaucoma = glaucoma::generate_simple(resolution, severity);
        maps.push(glaucoma);
    }

    // achromatopsia
    if let Some(Value::Bool(true)) = params.get("achromatopsia_onoff") {
        let severity = params.get("achromatopsia_int").unwrap().as_f64().unwrap() as u64;
        let achromatopsia = colorblindness::generate_achromatopsia(resolution, severity);
        maps.push(achromatopsia);
    }

    // nyctalopia
    if let Some(Value::Bool(true)) = params.get("nyctalopia_onoff") {
        let severity = params.get("nyctalopia_int").unwrap().as_f64().unwrap() as u64;
        let nyctalopia = nyctalopia::generate(resolution, severity);
        maps.push(nyctalopia);
    }

    // colorblindness
    if let Some(Value::Bool(true)) = params.get("colorblindness_onoff") {
        let ctype = params.get("colorblindness_type").unwrap().as_f64().unwrap() as u64;
        let severity = params.get("colorblindness_int").unwrap().as_f64().unwrap() as u64;
        let colorblindness = colorblindness::generate_colorblindness(resolution, ctype, severity);
        maps.push(colorblindness);
    }

    // macular degeneration
    if let Some(Value::Bool(true)) = params.get("maculardegeneration_onoff") {
        if let Some(Value::Bool(true)) = params.get("maculardegeneration_veasy") {
            // parameters set in easy easy mode
            let severity = params
                .get("maculardegeneration_inteasy")
                .unwrap()
                .as_f64()
                .unwrap() as u8;
            let macular_degeneration = macular_degeneration::generate_simple(resolution, severity);
            maps.push(macular_degeneration);
        } else if let Some(Value::Bool(true)) = params.get("maculardegeneration_vadvanced") {
            // parameters set in advanced mode
            let radius = params
                .get("maculardegeneration_radius")
                .unwrap()
                .as_f64()
                .unwrap();
            let severity = params
                .get("maculardegeneration_intadvanced")
                .unwrap()
                .as_f64()
                .unwrap();
            // interpret parameters
            let radius = radius * (resolution.0 as f64) / 300.0;
            let severity = 1.0 - 0.5 * (1.0 - severity / 100.0).powi(2);
            let macular_degeneration = macular_degeneration::generate(resolution, radius, severity);
            maps.push(macular_degeneration);
        }
    }

    // receptor density
    if let Some(Value::Bool(true)) = params.get("receptordensity_onoff") {
        let receptor_density = receptor_density::generate(resolution);
        maps.push(receptor_density);
    }

    merge_maps(maps, resolution)
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

    image::ImageRgba8(merged).raw_pixels().into_boxed_slice()
}
