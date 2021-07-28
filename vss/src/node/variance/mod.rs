use std::borrow::BorrowMut;

use super::*;
use gfx;
use gfx::format::Rgba32F;

gfx_defines! {
    pipeline pipe {
        u_resolution: gfx::Global<[f32; 2]> = "u_resolution",
        s_color: gfx::TextureSampler<[f32; 4]> = "s_color",
        s_original: gfx::TextureSampler<[f32; 4]> = "s_original",
        rt_color: gfx::RenderTarget<ColorFormat> = "rt_color",
        rt_measure: gfx::RenderTarget<ColorFormat> = "rt_measure",
        s_deflection: gfx::TextureSampler<[f32; 4]> = "s_deflection",
        rt_deflection: gfx::RenderTarget<Rgba32F> = "rt_deflection",
        s_color_change: gfx::TextureSampler<[f32; 4]> = "s_color_change",
        rt_color_change: gfx::RenderTarget<Rgba32F> = "rt_color_change",
        s_color_uncertainty: gfx::TextureSampler<[f32; 4]> = "s_color_uncertainty",
        rt_color_uncertainty: gfx::RenderTarget<Rgba32F> = "rt_color_uncertainty",
        s_covariances: gfx::TextureSampler<[f32; 4]> = "s_covariances",
        rt_covariances: gfx::RenderTarget<Rgba32F> = "rt_covariances",
        u_track_error: gfx::Global<i32> = "u_track_error",
        u_show_variance: gfx::Global<i32> = "u_show_variance",
        u_color_space: gfx::Global<i32> = "u_color_space",
        u_variance_measure: gfx::Global<i32> = "u_variance_measure",
    }
}

pub struct VarianceMeasure {
    pso: gfx::PipelineState<Resources, pipe::Meta>,
    pso_data: pipe::Data<Resources>,
    last_info: f32,
}

impl VarianceMeasure{}

impl Node for VarianceMeasure {
    fn new(window: &Window) -> Self {
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
                u_show_variance: 3,
                u_color_space: 2,
                u_variance_measure: 4,
            },
            last_info: 0.0,
        }
    }

    fn negociate_slots(&mut self, window: &Window, slots: NodeSlots) -> NodeSlots {
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
            
        let (color, _) = create_texture_render_target::<ColorFormat>(
            &mut window.factory().borrow_mut(),
            self.pso_data.u_resolution[0] as u32,
            self.pso_data.u_resolution[1] as u32,
        );
        self.pso_data.rt_measure = color;

        slots
    }

    fn negociate_slots_wk(&mut self, window: &Window, slots: NodeSlots, well_known: &WellKnownSlots) -> NodeSlots{
        self.pso_data.s_original = well_known.get_original().expect("Nah, no original image?");
        self.negociate_slots(window, slots)
    }

    fn input(&mut self, perspective: &EyePerspective, vis_param: &VisualizationParameters) -> EyePerspective {
        self.pso_data.u_track_error = vis_param.has_to_track_error() as i32;
        //self.pso_data.u_show_variance =  ((vis_param.vis_type.base_image) as u32) as i32;
        perspective.clone()
    }

    fn render(&mut self, window: &Window) {
        let mut encoder = window.encoder().borrow_mut();
        encoder.draw(&gfx::Slice::from_vertex_count(6), &self.pso, &self.pso_data);
        self.last_info += window.delta_t()/1000000.0;
        if self.last_info > 1.0{
            self.last_info -= 1.0;

            use gfx::format::Formatted;
            use gfx::memory::Typed;

            let factory = &mut window.factory().borrow_mut();
            let width = self.pso_data.u_resolution[0] as u32;
            let height = self.pso_data.u_resolution[1] as u32;

            // Schedule download.
            let download = factory
                .create_download_buffer::<[u8; 4]>((width * height) as usize)
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
                        format: ColorFormat::get_format(),
                        mipmap: 0,
                    },
                    download.raw(),
                    0,
                )
                .unwrap();

            // Flush before reading the buffers to prevent panics.
            window.flush(encoder.borrow_mut());

            // Copy to buffers.
            let mut sum_loss = 0.0;
            let reader = factory.read_mapping(&download).unwrap();
            for row in reader.chunks(width as usize).rev() {
                for pixel in row.iter() {
                    sum_loss += (pixel[0] as f32)/255.0;
                }
            }
            
            
            println!("Total Loss: {:?}\t Avg Loss: {:?}", sum_loss, sum_loss/(download.len() as f32 * 4.0));
        }
    }
}
