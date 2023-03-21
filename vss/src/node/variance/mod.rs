use super::*;

use std::borrow::BorrowMut;
use std::collections::HashMap;
use std::io::Write;
use std::fs::File;

// gfx_defines! {
//     pipeline pipe {
//         u_resolution: gfx::Global<[f32; 2]> = "u_resolution",
//         s_color: gfx::TextureSampler<[f32; 4]> = "s_color",
//         s_original: gfx::TextureSampler<[f32; 4]> = "s_original",
//         rt_color: gfx::RenderTarget<ColorFormat> = "rt_color",
//         rt_measure: gfx::RenderTarget<HighpFormat> = "rt_measure",
//         s_deflection: gfx::TextureSampler<[f32; 4]> = "s_deflection",
//         rt_deflection: gfx::RenderTarget<Rgba32F> = "rt_deflection",
//         s_color_change: gfx::TextureSampler<[f32; 4]> = "s_color_change",
//         rt_color_change: gfx::RenderTarget<Rgba32F> = "rt_color_change",
//         s_color_uncertainty: gfx::TextureSampler<[f32; 4]> = "s_color_uncertainty",
//         rt_color_uncertainty: gfx::RenderTarget<Rgba32F> = "rt_color_uncertainty",
//         s_covariances: gfx::TextureSampler<[f32; 4]> = "s_covariances",
//         rt_covariances: gfx::RenderTarget<Rgba32F> = "rt_covariances",
//         u_track_error: gfx::Global<i32> = "u_track_error",
//         u_show_variance: gfx::Global<u32> = "u_show_variance",
//         u_variance_metric: gfx::Global<u32> = "u_variance_metric",
//         u_color_space: gfx::Global<u32> = "u_color_space",
//     }
// }

struct Uniforms{
    resolution: [f32; 2],
    track_error: i32,
    show_variance: u32,
    variance_metric: u32,
    color_space: u32,
}

pub struct VarianceMeasure {
    pipeline: wgpu::RenderPipeline,
    uniforms: ShaderUniforms<Uniforms>,
    sources_bind_group: wgpu::BindGroup,
    original_bind_group: wgpu::BindGroup,
    targets: ColorTargets,
    rt_measurement: RenderTexture,

    pub log_file: Option<File>,
    last_info: f32,
}

impl VarianceMeasure{
    pub fn new(surface: &Surface) -> Self {
        let device = surface.device().borrow_mut();
        let queue = surface.queue().borrow_mut();

        let uniforms = ShaderUniforms::new(&device, 
            Uniforms{
                resolution: [1.0, 1.0],
                track_error: 0,
                show_variance: 0,
                variance_metric: 0,
                color_space: 0,
            }
        );

        let (sources_bind_group_layout, sources_bind_group) = create_color_sources_bind_group(&device, &queue, "Variance");

        let original_tex = placeholder_texture(&device, &queue, Some("VarianceNode s_original")).unwrap();
        let(original_bind_group_layout, original_bind_group) = original_tex.create_bind_group(&device);

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("VarianceNode Shader"),
            source: wgpu::ShaderSource::Wgsl(concat!(
                include_str!("../vert.wgsl"),
                include_str!("mod.wgsl")).into()),
        });

        let pipeline = create_render_pipeline(
            &device,
            &[&shader, &shader],
            &["vs_main", "fs_main"],
            &[&uniforms.bind_group_layout, &sources_bind_group_layout, &original_bind_group_layout],
            &all_color_states(),
            None,
            Some("VarianceNode Render Pipeline"));
    
        // let (_, _, rt_measure) = factory.create_render_target(1, 1).unwrap();

        VarianceMeasure {
            pipeline,
            uniforms,
            sources_bind_group,
            original_bind_group,
            targets: ColorTargets::new(&device, "VarianceNode"),
            rt_measurement: placeholder_highp_rt(&device, Some("VarianceNode rt_measurement (placeholder)")),
            log_file: None,
            last_info: 1.0,
        }
    }


/*  fn measure_variance(&mut self, surface: &Surface) -> (f32, f32){
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
    }*/
}


impl Node for VarianceMeasure {
    
    fn negociate_slots(&mut self, surface: &Surface, slots: NodeSlots) -> NodeSlots {
        let slots = slots.to_color_input(surface).to_color_output(surface, "VarianceNode");
        self.uniforms.data.resolution = slots.output_size_f32();

        let device = surface.device().borrow_mut();

        self.sources_bind_group = slots.as_all_colors_source(&device);
        self.targets = slots.as_all_colors_target();

        self.rt_measurement = create_render_texture(
            &device,
            self.uniforms.data.resolution[0] as u32,
            self.uniforms.data.resolution[1] as u32,
            HIGHP_FORMAT,
            create_sampler_nearest(&device),
            Some("VarianceNode rt_measurement")
        );

        slots
    }

    fn input(&mut self, perspective: &EyePerspective, vis_param: &VisualizationParameters) -> EyePerspective {
        self.uniforms.data.track_error = vis_param.has_to_track_error() as i32;        
        self.uniforms.data.show_variance =  vis_param.measure_variance;
        self.uniforms.data.variance_metric =  vis_param.variance_metric;
        self.uniforms.data.color_space =  vis_param.variance_color_space;
        perspective.clone()
    }

    fn render(&mut self, surface: &surface::Surface, encoder: &mut CommandEncoder, screen: Option<&RenderTexture>) {
        if self.log_file.is_some(){
            /*self.pso_data.u_show_variance =  2;
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
            write!(self.log_file.as_ref().unwrap(), "{:?}, {:?}\n", sum, avg).unwrap();*/
        }else{
            self.uniforms.update(&surface.queue().borrow_mut());

            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Variance render_pass"),
                color_attachments: &self.targets.color_attachments(screen),
                depth_stencil_attachment: None,
            });
        
            render_pass.set_pipeline(&self.pipeline);
            render_pass.set_bind_group(0, &self.uniforms.bind_group, &[]);
            render_pass.set_bind_group(1, &self.sources_bind_group, &[]);
            render_pass.set_bind_group(2, &self.original_bind_group, &[]);
            render_pass.draw(0..6, 0..1);
            /* self.last_info += window.delta_t()/1000000.0;
            if self.last_info >= 1.0 && self.pso_data.u_show_variance > 0 {
                self.last_info = 0.0;

                let (sum, avg) = self.measure_variance(window);
                if self.pso_data.u_variance_metric == 6 {
                    println!("Total amount of colors: {:?}\t Avg histogram variance: {:?}", sum, avg);
                }else{
                    println!("Variance sum: {:?}\t Avg variance per pixel: {:?}", sum, avg);
                }
            }*/
        }
    }
}
