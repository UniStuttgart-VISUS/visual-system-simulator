mod retina_map;

use self::retina_map::generate_retina_map;
use super::*;
use cgmath::{Matrix4, Point3, SquareMatrix, Vector3};
use gfx;
use gfx::format::Rgba32F;


gfx_defines! {
    pipeline pipe {
        u_resolution: gfx::Global<[f32; 2]> = "u_resolution",
        u_proj: gfx::Global<[[f32; 4];4]> = "u_proj",
        s_source: gfx::TextureSampler<[f32; 4]> = "s_color",
        s_retina: gfx::TextureSampler<[f32; 4]> = "s_retina",
        rt_color: gfx::RenderTarget<ColorFormat> = "rt_color",
        s_deflection: gfx::TextureSampler<[f32; 4]> = "s_deflection",
        rt_deflection: gfx::RenderTarget<Rgba32F> = "rt_deflection",
        s_color_change: gfx::TextureSampler<[f32; 4]> = "s_color_change",
        rt_color_change: gfx::RenderTarget<Rgba32F> = "rt_color_change",
        s_color_uncertainty: gfx::TextureSampler<[f32; 4]> = "s_color_uncertainty",
        rt_color_uncertainty: gfx::RenderTarget<Rgba32F> = "rt_color_uncertainty",
        s_covariances: gfx::TextureSampler<[f32; 4]> = "s_covariances",
        rt_covariances: gfx::RenderTarget<Rgba32F> = "rt_covariances",
    }
}

pub struct Retina {
    pso: gfx::PipelineState<Resources, pipe::Meta>,
    pso_data: pipe::Data<Resources>,
}

impl Node for Retina {
    fn new(window: &Window) -> Self {
        let mut factory = window.factory().borrow_mut();
        let pso = factory
            .create_pipeline_simple(
                &include_glsl!("../mod.vert"),
                &include_glsl!("mod.frag"),
                pipe::new(),
            )
            .unwrap();

        let (_, mask_view) = load_cubemap_from_bytes(&mut factory, &[&[255; 4]; 6], 1).unwrap();
        let sampler = factory.create_sampler_linear();

        let (_, src, dst) = factory.create_render_target(1, 1).unwrap();
        let (_, s_deflection, rt_deflection) = factory.create_render_target(1, 1).unwrap();
        let (_, s_color_change, rt_color_change) = factory.create_render_target(1, 1).unwrap();
        let (_, s_color_uncertainty, rt_color_uncertainty) = factory.create_render_target(1, 1).unwrap();
        let (_, s_covariances, rt_covariances) = factory.create_render_target(1, 1).unwrap();

        Retina {
            pso,
            pso_data: pipe::Data {
                u_resolution: [1.0, 1.0],
                u_proj: Matrix4::from_scale(1.0).into(),
                s_source: (src, sampler.clone()),
                s_retina: (mask_view, sampler.clone()),
                rt_color: dst,
                s_deflection:(s_deflection, sampler.clone()),
                rt_deflection,
                s_color_change:(s_color_change, sampler.clone()),
                rt_color_change,
                s_color_uncertainty:(s_color_uncertainty, sampler.clone()),
                rt_color_uncertainty,
                s_covariances: (s_covariances, sampler.clone()),
                rt_covariances
            },
        }
    }

    fn negociate_slots(&mut self, window: &Window, slots: NodeSlots) -> NodeSlots {
        let slots = slots.to_color_input(window).to_color_output(window);
        self.pso_data.u_resolution = slots.output_size_f32();
        self.pso_data.s_source = slots.as_color_view();
        self.pso_data.rt_color = slots.as_color();
        self.pso_data.s_deflection = slots.as_deflection_view();
        self.pso_data.rt_deflection = slots.as_deflection();
        self.pso_data.s_color_change = slots.as_color_change_view();
        self.pso_data.rt_color_change = slots.as_color_change();  
        self.pso_data.s_color_uncertainty = slots.as_color_uncertainty_view();
        self.pso_data.rt_color_uncertainty = slots.as_color_uncertainty();
        self.pso_data.s_covariances = slots.as_covariances_view();
        self.pso_data.rt_covariances = slots.as_covariances();
        slots
    }

    fn update_values(&mut self, window: &Window, values: &ValueMap) {
        let mut factory = window.factory().borrow_mut();
        let mut image_data = Vec::new();
        if let Some(Value::Image(retina_map_pos_x_path)) = values.get("retina_map_pos_x_path") {
            image_data.push(load(retina_map_pos_x_path));
        }
        if let Some(Value::Image(retina_map_neg_x_path)) = values.get("retina_map_neg_x_path") {
            image_data.push(load(retina_map_neg_x_path));
        }
        if let Some(Value::Image(retina_map_pos_y_path)) = values.get("retina_map_pos_y_path") {
            image_data.push(load(retina_map_pos_y_path));
        }
        if let Some(Value::Image(retina_map_neg_y_path)) = values.get("retina_map_neg_y_path") {
            image_data.push(load(retina_map_neg_y_path));
        }
        if let Some(Value::Image(retina_map_pos_z_path)) = values.get("retina_map_pos_z_path") {
            image_data.push(load(retina_map_pos_z_path));
        }
        if let Some(Value::Image(retina_map_neg_z_path)) = values.get("retina_map_neg_z_path") {
            image_data.push(load(retina_map_neg_z_path));
        }
        if image_data.len() == 6 {
            let (_, retinamap_view) = load_cubemap(&mut factory, image_data).unwrap();
            self.pso_data.s_retina = (retinamap_view, self.pso_data.s_retina.clone().1);
        } else {
            let proj_val = Value::Matrix(Matrix4::from_scale(1.0));
            let projection = values.get("proj_matrix").unwrap_or(&proj_val).as_matrix().unwrap();
            let res_x = (self.pso_data.u_resolution[0] * 2.0 * projection[0][0]) as f32;
            let res_y = (self.pso_data.u_resolution[1] * 2.0 * projection[1][1]) as f32;
            let resolution = (res_x.max(res_y) + 1.0) as u32;
            let cubemap_resolution = (
                resolution,
                resolution,
            );

            //orientations directly taken from https://www.khronos.org/opengl/wiki/Cubemap_Texture
            let retina_map_pos_x = generate_retina_map(cubemap_resolution, &[-Vector3::unit_z(), -Vector3::unit_y(),  Vector3::unit_x()], &values);
            let retina_map_neg_x = generate_retina_map(cubemap_resolution, &[ Vector3::unit_z(), -Vector3::unit_y(), -Vector3::unit_x()], &values);
            let retina_map_pos_y = generate_retina_map(cubemap_resolution, &[ Vector3::unit_x(),  Vector3::unit_z(),  Vector3::unit_y()], &values);
            let retina_map_neg_y = generate_retina_map(cubemap_resolution, &[ Vector3::unit_x(), -Vector3::unit_z(), -Vector3::unit_y()], &values);
            let retina_map_pos_z = generate_retina_map(cubemap_resolution, &[ Vector3::unit_x(), -Vector3::unit_y(),  Vector3::unit_z()], &values);
            let retina_map_neg_z = generate_retina_map(cubemap_resolution, &[-Vector3::unit_x(), -Vector3::unit_y(), -Vector3::unit_z()], &values);
            //save latest retina map
            //let _ = image::save_buffer(&Path::new("last.retina_pos_x.png"), &retina_map_pos_x, cubemap_resolution.0, cubemap_resolution.1, image::ColorType::Rgba8);
            //let _ = image::save_buffer(&Path::new("last.retina_neg_x.png"), &retina_map_neg_x, cubemap_resolution.0, cubemap_resolution.1, image::ColorType::Rgba8);
            //let _ = image::save_buffer(&Path::new("last.retina_pos_y.png"), &retina_map_pos_y, cubemap_resolution.0, cubemap_resolution.1, image::ColorType::Rgba8);
            //let _ = image::save_buffer(&Path::new("last.retina_neg_y.png"), &retina_map_neg_y, cubemap_resolution.0, cubemap_resolution.1, image::ColorType::Rgba8);
            //let _ = image::save_buffer(&Path::new("last.retina_pos_z.png"), &retina_map_pos_z, cubemap_resolution.0, cubemap_resolution.1, image::ColorType::Rgba8);
            //let _ = image::save_buffer(&Path::new("last.retina_neg_z.png"), &retina_map_neg_z, cubemap_resolution.0, cubemap_resolution.1, image::ColorType::Rgba8);
            let (_, retinamap_view) = load_cubemap_from_bytes(
                &mut factory,
                &[&retina_map_pos_x, &retina_map_neg_x, &retina_map_pos_y, &retina_map_neg_y, &retina_map_pos_z, &retina_map_neg_z],
                cubemap_resolution.0,
            )
            .unwrap();

            self.pso_data.s_retina = (retinamap_view, self.pso_data.s_retina.clone().1);
        };
    }

    fn input(&mut self, perspective: &EyePerspective, _vis_param: &VisualizationParameters) -> EyePerspective {
        let gaze_rotation = Matrix4::look_to_lh(Point3::new(0.0, 0.0, 0.0), perspective.gaze, Vector3::unit_y());
        //let gaze_rotation = Matrix4::from_scale(1.0);
        self.pso_data.u_proj = (gaze_rotation * perspective.proj.invert().unwrap()).into();
        //self.pso_data.u_proj = (head.proj * (Matrix4::from_translation(-head.position) * head.view)).into();
        perspective.clone()
    }

    fn render(&mut self, window: &Window) {
        let mut encoder = window.encoder().borrow_mut();
        encoder.draw(&gfx::Slice::from_vertex_count(6), &self.pso, &self.pso_data);
    }
}
