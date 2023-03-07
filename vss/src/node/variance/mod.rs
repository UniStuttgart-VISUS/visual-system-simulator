use std::borrow::BorrowMut;
use std::collections::HashMap;
use std::io::Write;
use std::fs::File;

use super::*;
use gfx;
use gfx::format::Rgba32F;

gfx_defines! {
    pipeline pipe {
        u_resolution: gfx::Global<[f32; 2]> = "u_resolution",
        s_color: gfx::TextureSampler<[f32; 4]> = "s_color",
        s_original: gfx::TextureSampler<[f32; 4]> = "s_original",
        rt_color: gfx::RenderTarget<ColorFormat> = "rt_color",
        rt_measure: gfx::RenderTarget<HighpFormat> = "rt_measure",
        s_deflection: gfx::TextureSampler<[f32; 4]> = "s_deflection",
        rt_deflection: gfx::RenderTarget<Rgba32F> = "rt_deflection",
        s_color_change: gfx::TextureSampler<[f32; 4]> = "s_color_change",
        rt_color_change: gfx::RenderTarget<Rgba32F> = "rt_color_change",
        s_color_uncertainty: gfx::TextureSampler<[f32; 4]> = "s_color_uncertainty",
        rt_color_uncertainty: gfx::RenderTarget<Rgba32F> = "rt_color_uncertainty",
        s_covariances: gfx::TextureSampler<[f32; 4]> = "s_covariances",
        rt_covariances: gfx::RenderTarget<Rgba32F> = "rt_covariances",
        u_track_error: gfx::Global<i32> = "u_track_error",
        u_show_variance: gfx::Global<u32> = "u_show_variance",
        u_variance_metric: gfx::Global<u32> = "u_variance_metric",
        u_color_space: gfx::Global<u32> = "u_color_space",
    }
}

pub struct VarianceMeasure {
    pso: gfx::PipelineState<Resources, pipe::Meta>,
    pso_data: pipe::Data<Resources>,
    pub log_file: Option<File>,
    last_info: f32,
}

impl VarianceMeasure{
    fn measure_variance(&mut self, surface: &Surface) -> (f32, f32){
        use gfx::format::Formatted;
        use gfx::memory::Typed;

        let mut encoder = window.encoder().borrow_mut();
        let factory = &mut window.factory().borrow_mut();
        let width = self.pso_data.u_resolution[0] as u32;
        let height = self.pso_data.u_resolution[1] as u32;

        // Schedule download.
        let download = factory
            .create_download_buffer::<[f32; 3]>((width * height) as usize)
            .unwrap();
        encoder
            .copy_texture_to_buffer_raw(
                self.pso_data.rt_measure.raw().get_texture(),
                None,
                gfx::texture::RawImageInfo {
                    xoffset: 0,
                    yoffset: 0,
                    zoffset: 0,
                    width: width as u16,
                    height: height as u16,
                    depth: 0,
                    format: HighpFormat::get_format(),
                    mipmap: 0,
                },
                download.raw(),
                0,
            )
            .unwrap();

        // Flush before reading the buffers to prevent panics.
        window.flush(encoder.borrow_mut());

        if self.pso_data.u_variance_metric == 6 {
            assert!(self.pso_data.u_show_variance < 3, "Histogram can only be calculated with variance Type <Before> or <After>");
            assert!(self.pso_data.u_color_space == 2, "Histogram can only be calculated with color Type <ITP>");
            // calculate histogram
            let mut color_map = HashMap::new();
            let reader = factory.read_mapping(&download).unwrap();
            for row in reader.chunks(width as usize).rev() {
                for pixel in row.iter() {
                    let pixel_key = (((pixel[0] * 255.0) as u32) << 16) | (((pixel[1] * 255.0) as u32) << 8) | ((pixel[2] * 255.0) as u32);
                    color_map.entry(pixel_key).or_insert([pixel[0], pixel[1], pixel[2], 0.0])[3] += 1.0/(width as f32 * height as f32);
                }
            }
            let mut histogram_variance = 0.0;
            for (index, (_color_key, color)) in color_map.iter().enumerate(){
                for (index_inner, (_color_key_inner, color_inner)) in color_map.iter().enumerate(){
                    if index_inner >= index {break;}
                    let diff = (color_inner[0] - color[0], color_inner[1] - color[1], color_inner[2] - color[2]);
                    let length = (diff.0*diff.0 + diff.1*diff.1*0.5 + diff.2*diff.2).sqrt() as f64;
                    histogram_variance += length * 2.0 * (color[3] as f64 * color_inner[3] as f64);
                }
            }
            (color_map.len() as f32, histogram_variance as f32)
        }else{
            assert!((self.pso_data.u_variance_metric != 5) || (self.pso_data.u_color_space == 1), "Michelson Contrast can only be calculated with color Type <LAB>");
            // sum up variance
            let mut sum_variance = 0.0;
            let reader = factory.read_mapping(&download).unwrap();
            for row in reader.chunks(width as usize).rev() {
                for pixel in row.iter() {
                    sum_variance += pixel[0] as f32;
                }
            }
            (sum_variance, sum_variance/(download.len() as f32))
        }
    }
}


impl Node for VarianceMeasure {
    fn new(surface: &Surface) -> Self {
        let mut factory = window.factory().borrow_mut();

        let pso = factory
            .create_pipeline_simple(
                &include_glsl!("../mod.vert"),
                &include_glsl!("mod.frag"),
                pipe::new(),
            )
            .unwrap();

        let sampler = factory.create_sampler_linear();
        let (_, color_view) = load_texture_from_bytes(&mut factory, &[0; 4], 1, 1).unwrap();
        let (_, capture_view) = load_texture_from_bytes(&mut factory, &[0; 4], 1, 1).unwrap();
        let (_, _, rt_color) = factory.create_render_target(1, 1).unwrap();
        let (_, _, rt_measure) = factory.create_render_target(1, 1).unwrap();
        let (_, s_deflection, rt_deflection) = factory.create_render_target(1, 1).unwrap();
        let (_, s_color_change, rt_color_change) = factory.create_render_target(1, 1).unwrap();
        let (_, s_color_uncertainty, rt_color_uncertainty) = factory.create_render_target(1, 1).unwrap();
        let (_, s_covariances, rt_covariances) = factory.create_render_target(1, 1).unwrap();

        VarianceMeasure {
            pso,
            pso_data: pipe::Data {
                u_resolution: [1.0, 1.0],
                s_color: (color_view, sampler.clone()),
                s_original: (capture_view.clone(), sampler.clone()),
                rt_color,
                rt_measure,
                s_deflection:(s_deflection, sampler.clone()),
                rt_deflection,
                s_color_change:(s_color_change, sampler.clone()),
                rt_color_change,
                s_color_uncertainty:(s_color_uncertainty, sampler.clone()),
                rt_color_uncertainty,
                s_covariances: (s_covariances, sampler.clone()),
                rt_covariances,
                u_track_error: 0,
                u_show_variance: 0,
                u_variance_metric: 0,
                u_color_space: 0,
            },
            log_file: None,
            last_info: 1.0,
        }
    }

    fn negociate_slots(&mut self, surface: &Surface, slots: NodeSlots) -> NodeSlots {
        let slots = slots
            .to_color_input(window)
            .to_color_output(window);

        self.pso_data.u_resolution = slots.output_size_f32();
        self.pso_data.s_color = slots.as_color_view();
        self.pso_data.rt_color = slots.as_color();
        self.pso_data.s_deflection = slots.as_deflection_view();
        self.pso_data.rt_deflection = slots.as_deflection();
        self.pso_data.s_color_change = slots.as_color_change_view();
        self.pso_data.rt_color_change = slots.as_color_change();  
        self.pso_data.s_color_uncertainty = slots.as_color_uncertainty_view();
        self.pso_data.rt_color_uncertainty = slots.as_color_uncertainty();
        self.pso_data.s_covariances = slots.as_covariances_view();
        self.pso_data.rt_covariances = slots.as_covariances();
            
        let (color, _) = create_texture_render_target::<HighpFormat>(
            &mut window.factory().borrow_mut(),
            self.pso_data.u_resolution[0] as u32,
            self.pso_data.u_resolution[1] as u32,
        );
        self.pso_data.rt_measure = color;

        slots
    }

    fn input(&mut self, perspective: &EyePerspective, vis_param: &VisualizationParameters) -> EyePerspective {
        self.pso_data.u_track_error = vis_param.has_to_track_error() as i32;
        self.pso_data.u_show_variance =  vis_param.measure_variance;
        self.pso_data.u_variance_metric =  vis_param.variance_metric;
        self.pso_data.u_color_space =  vis_param.variance_color_space;
        perspective.clone()
    }

    fn render(&mut self, surface: &Surface) {
        if self.log_file.is_some(){
            self.pso_data.u_show_variance =  2;
            self.pso_data.u_variance_metric =  4;
            self.pso_data.u_color_space =  2;
            window.encoder().borrow_mut().draw(&gfx::Slice::from_vertex_count(6), &self.pso, &self.pso_data);
            let (sum, avg) = self.measure_variance(window);
            write!(self.log_file.as_ref().unwrap(), "{:?}, {:?}, ", sum, avg).unwrap();

            self.pso_data.u_variance_metric =  5;
            self.pso_data.u_color_space =  1;
            window.encoder().borrow_mut().draw(&gfx::Slice::from_vertex_count(6), &self.pso, &self.pso_data);
            let (sum, avg) = self.measure_variance(window);
            write!(self.log_file.as_ref().unwrap(), "{:?}, {:?}, ", sum, avg).unwrap();

            self.pso_data.u_variance_metric =  6;
            self.pso_data.u_color_space =  2;
            window.encoder().borrow_mut().draw(&gfx::Slice::from_vertex_count(6), &self.pso, &self.pso_data);
            let (sum, avg) = self.measure_variance(window);
            write!(self.log_file.as_ref().unwrap(), "{:?}, {:?}\n", sum, avg).unwrap();
        }else{
            window.encoder().borrow_mut().draw(&gfx::Slice::from_vertex_count(6), &self.pso, &self.pso_data);
            self.last_info += window.delta_t()/1000000.0;
            if self.last_info >= 1.0 && self.pso_data.u_show_variance > 0 {
                self.last_info = 0.0;

                let (sum, avg) = self.measure_variance(window);
                if self.pso_data.u_variance_metric == 6 {
                    println!("Total amount of colors: {:?}\t Avg histogram variance: {:?}", sum, avg);
                }else{
                    println!("Variance sum: {:?}\t Avg variance per pixel: {:?}", sum, avg);
                }
            }
        }
    }
}
