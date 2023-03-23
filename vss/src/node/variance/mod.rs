use wgpu::{Buffer, BufferView};

use super::*;

use std::{collections::HashMap, mem::size_of, time::Instant};

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
    target_measurement: RenderTexture,
    download_buffer: Buffer,
    buffer_dimensions: BufferDimensions,
    last_info: Instant,
    should_download: bool,
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
            &[
                blended_color_state(COLOR_FORMAT),
                simple_color_state(HIGHP_FORMAT),
                simple_color_state(HIGHP_FORMAT),
                simple_color_state(HIGHP_FORMAT),
                simple_color_state(HIGHP_FORMAT),
                simple_color_state(HIGHP_FORMAT),
            ],
            None,
            Some("VarianceNode Render Pipeline")
        );
        
        let buffer_dimensions = BufferDimensions::new(1 as usize, 1 as usize, size_of::<[f32; 4]>());
        let download_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Download Node Placeholder Buffer"),
            size: (buffer_dimensions.padded_bytes_per_row * buffer_dimensions.height) as u64,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
    
        VarianceMeasure {
            pipeline,
            uniforms,
            sources_bind_group,
            original_bind_group,
            targets: ColorTargets::new(&device, "VarianceNode"),
            target_measurement: placeholder_highp_rt(&device, Some("VarianceNode target_measurement (placeholder)")),
            download_buffer,
            buffer_dimensions,
            last_info: Instant::now(),
            should_download: false,
        }
    }


    fn measure_variance(&mut self, surface: &Surface) -> (f32, f32){
        let device = surface.device().borrow_mut();

        // Note that we're not calling `.await` here.
        let buffer_slice = self.download_buffer.slice(..);
        
        let (sender, _receiver) = futures_intrusive::channel::shared::oneshot_channel();
        buffer_slice.map_async(wgpu::MapMode::Read, move |v| {sender.send(v).unwrap();} );

        device.poll(wgpu::Maintain::Wait);

        let padded_buffer = buffer_slice.get_mapped_range();

        if self.uniforms.data.variance_metric == 6 {
            assert!(self.uniforms.data.show_variance < 3, "Histogram can only be calculated with variance Type <Before> or <After>");
            assert!(self.uniforms.data.color_space == 2, "Histogram can only be calculated with color Type <ITP>");

            // calculate histogram
            let mut color_map = HashMap::new();
            let width = self.buffer_dimensions.width;
            let height = self.buffer_dimensions.height;
            for chunk in padded_buffer.chunks(self.buffer_dimensions.padded_bytes_per_row) {
                let pixels: &[f32] = unsafe { std::slice::from_raw_parts(chunk.as_ptr() as *const f32, self.buffer_dimensions.unpadded_bytes_per_row / 4)};
                for i in (0 .. self.buffer_dimensions.unpadded_bytes_per_row / 4).step_by(4) {
                    let pixel_key = (((pixels[i+0] * 255.0) as u32) << 16) | (((pixels[i+1] * 255.0) as u32) << 8) | ((pixels[i+2] * 255.0) as u32);
                    color_map.entry(pixel_key).or_insert([pixels[i+0], pixels[i+1], pixels[i+2], 0.0])[3] += 1.0/(width as f32 * height as f32);
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
            assert!((self.uniforms.data.variance_metric != 5) || (self.uniforms.data.color_space == 1), "Michelson Contrast can only be calculated with color Type <LAB>");
            // sum up variance
            let mut sum_variance = 0.0;
            for chunk in padded_buffer.chunks(self.buffer_dimensions.padded_bytes_per_row) {
                let pixels: &[f32] = unsafe { std::slice::from_raw_parts(chunk.as_ptr() as *const f32, self.buffer_dimensions.unpadded_bytes_per_row / 4)};
                for i in (0 .. self.buffer_dimensions.unpadded_bytes_per_row / 4).step_by(4) {
                    sum_variance += pixels[i] as f32;
                }
            }
            (sum_variance, sum_variance/((self.buffer_dimensions.width * self.buffer_dimensions.height) as f32))
        }
    }
}


impl Node for VarianceMeasure {
    
    fn negociate_slots(&mut self, surface: &Surface, slots: NodeSlots) -> NodeSlots {
        let slots = slots.to_color_input(surface).to_color_output(surface, "VarianceNode");
        self.uniforms.data.resolution = slots.output_size_f32();

        let device = surface.device().borrow_mut();

        self.sources_bind_group = slots.as_all_colors_source(&device);
        self.targets = slots.as_all_colors_target();

        self.target_measurement = create_render_texture(
            &device,
            self.uniforms.data.resolution[0] as u32,
            self.uniforms.data.resolution[1] as u32,
            HIGHP_FORMAT,
            create_sampler_nearest(&device),
            Some("VarianceNode target_measurement")
        );

        self.buffer_dimensions = BufferDimensions::new(self.uniforms.data.resolution[0] as usize, self.uniforms.data.resolution[1] as usize, size_of::<[f32; 4]>());
        let download_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Download Node Buffer"),
            size: (self.buffer_dimensions.padded_bytes_per_row * self.buffer_dimensions.height) as u64,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        self.download_buffer = download_buffer;

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
        self.uniforms.update(&surface.queue().borrow_mut());

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Variance render_pass"),
                color_attachments: &[
                    screen.unwrap_or(&self.targets.rt_color).to_color_attachment(),
                    self.targets.rt_deflection.to_color_attachment(),
                    self.targets.rt_color_change.to_color_attachment(),
                    self.targets.rt_color_uncertainty.to_color_attachment(),
                    self.targets.rt_covariances.to_color_attachment(),
                    self.target_measurement.to_color_attachment(),
                ],
                depth_stencil_attachment: None,
            });
        
            render_pass.set_pipeline(&self.pipeline);
            render_pass.set_bind_group(0, &self.uniforms.bind_group, &[]);
            render_pass.set_bind_group(1, &self.sources_bind_group, &[]);
            render_pass.set_bind_group(2, &self.original_bind_group, &[]);
            render_pass.draw(0..6, 0..1);
        }

        if Instant::now().duration_since(self.last_info).as_secs_f32() >= 1.0 && self.uniforms.data.show_variance > 0 {
            self.last_info = Instant::now();

            // Schedule download.
            encoder.copy_texture_to_buffer(
                self.target_measurement.as_texture().texture.as_image_copy(),
                wgpu::ImageCopyBuffer {
                    buffer: &self.download_buffer,
                    layout: wgpu::ImageDataLayout {
                        offset: 0,
                        bytes_per_row: Some(
                            std::num::NonZeroU32::new(self.buffer_dimensions.padded_bytes_per_row as u32)
                                .unwrap(),
                        ),
                        rows_per_image: None,
                    },
                },
                wgpu::Extent3d {
                    width: self.buffer_dimensions.width as u32,
                    height: self.buffer_dimensions.height as u32,
                    depth_or_array_layers: 1,
                },
            );
            self.should_download = true;
        }
    }

    fn post_render(&mut self, surface: &Surface) {
        if(self.should_download){
            let (sum, avg) = self.measure_variance(&surface);
            self.download_buffer.unmap();
            self.should_download = false;
            if self.uniforms.data.variance_metric == 6 {
                println!("Total amount of colors: {:?}\t Avg histogram variance: {:?}", sum, avg);
            }else{
                println!("Variance sum: {:?}\t Avg variance per pixel: {:?}", sum, avg);
            }
        }
    }
}
