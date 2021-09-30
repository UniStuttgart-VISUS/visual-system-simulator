use core::f32;
use std::borrow::Borrow;
use std::borrow::BorrowMut;

use super::*;
use gfx;
use cgmath::Matrix4;
use cgmath::Rad;
use eframe::egui::*;

gfx_defines! {
    pipeline pipe {
        u_stereo: gfx::Global<i32> = "u_stereo",
        u_resolution_in: gfx::Global<[f32; 2]> = "u_resolution_in",
        u_resolution_out: gfx::Global<[f32; 2]> = "u_resolution_out",
        s_source: gfx::TextureSampler<[f32; 4]> = "s_color",
        rt_color: gfx::RenderTarget<ColorFormat> = "rt_color",
        u_vis_type: gfx::Global<i32> = "u_vis_type",
        u_heat_scale: gfx::Global<f32> = "u_heat_scale",
        u_dir_calc_scale: gfx::Global<f32> = "u_dir_calc_scale",
        s_deflection: gfx::TextureSampler<[f32; 4]> = "s_deflection",
        s_color_change: gfx::TextureSampler<[f32; 4]> = "s_color_change",
        s_color_uncertainty: gfx::TextureSampler<[f32; 4]> = "s_color_uncertainty",
        s_original: gfx::TextureSampler<[f32; 4]> = "s_original",
        s_covariances: gfx::TextureSampler<[f32; 4]> = "s_covariances",
        u_flow_idx: gfx::Global<i32> = "u_flow_idx",
        u_hive_rotation: gfx::Global<[[f32; 4];4]> = "u_hive_rotation",
        u_hive_position: gfx::Global<[f32; 2]> = "u_hive_position",
        u_hive_visible: gfx::Global<i32> = "u_hive_visible",

        u_base_image: gfx::Global<i32> = "u_base_image",
        u_combination_function: gfx::Global<i32> = "u_combination_function",
        u_mix_type: gfx::Global<i32> = "u_mix_type",
        u_colormap_type: gfx::Global<i32> = "u_colormap_type",
    }

    vertex Vertex {
        pos: [f32; 2] = "a_pos",
        uv: [f32; 2] = "a_uv",
        color: [f32; 4] = "a_col",
    }

    pipeline gui_pipe {
        vbuf: gfx::VertexBuffer<Vertex> = (),
        u_texture: gfx::TextureSampler<[f32; 4]> = "u_tex",
        u_resolution_in: gfx::Global<[f32; 2]> = "u_resolution_in",
        u_resolution_out: gfx::Global<[f32; 2]> = "u_resolution_out",
        rt_color: gfx::BlendTarget<ColorFormat> = ("rt_color", gfx::state::ColorMask::all(), gfx::preset::blend::ALPHA),
    }
}

const TRIANGLE: [Vertex; 3] = [
    Vertex { pos: [ -0.5, -0.5 ], uv: [ 0.0, 0.0 ], color: [1.0, 0.0, 0.0, 1.0] },
    Vertex { pos: [  0.5, -0.5 ], uv: [ 0.0, 0.0 ], color: [0.0, 1.0, 0.0, 1.0] },
    Vertex { pos: [  0.0,  0.5 ], uv: [ 0.0, 0.0 ], color: [0.0, 0.0, 1.0, 1.0] }
];

pub struct Display {
    pso: gfx::PipelineState<Resources, pipe::Meta>,
    pso_data: pipe::Data<Resources>,
    gui_pso: gfx::PipelineState<Resources, gui_pipe::Meta>,
    gui_pso_data: gui_pipe::Data<Resources>,
    hive_rot: Matrix4<f32>,
    resolution: [f32; 2],
    gui_context: eframe::egui::CtxRef,
    //gui_texture: gfx::handle::ShaderResourceView<Resources, [f32; 4]>,
    gui_texture_version: u64,
    gui_meshes: Vec<eframe::egui::ClippedMesh>
}

impl Node for Display {
    fn new(window: &window::Window) -> Self {
        let mut factory = window.factory().borrow_mut();

        let pso = factory
            .create_pipeline_simple(
                &include_glsl!("mod.vert"),
                &include_glsl!("mod.frag"),
                pipe::new(),
            )
            .unwrap();
        
        let gui_pso = factory
            .create_pipeline_simple(
                &include_glsl!("gui.vert"),
                &include_glsl!("gui.frag"),
                gui_pipe::new(),
            )
            .unwrap();

        let sampler = factory.create_sampler_linear();
        let (_, src, dst) = factory.create_render_target(1, 1).unwrap();

        // add deflection view
        let (_, s_deflection, _): (
            _,
            _,
            gfx::handle::RenderTargetView<gfx_device_gl::Resources, [f32; 4]>,
        ) = factory.create_render_target(1, 1).unwrap();
        let (_, s_color_change, _): (
            _,
            _,
            gfx::handle::RenderTargetView<gfx_device_gl::Resources, [f32; 4]>,
        ) = factory.create_render_target(1, 1).unwrap();
        let (_, s_color_uncertainty, _): (
            _,
            _,
            gfx::handle::RenderTargetView<gfx_device_gl::Resources, [f32; 4]>,
        ) = factory.create_render_target(1, 1).unwrap();
        let (_, s_original, _): (
            _,
            _,
            gfx::handle::RenderTargetView<gfx_device_gl::Resources, [f32; 4]>,
        ) = factory.create_render_target(1, 1).unwrap();

        let (_, s_covariances, _):(
            _,
            _,
            gfx::handle::RenderTargetView<gfx_device_gl::Resources, [f32; 4]>,
        ) = factory.create_render_target(1, 1).unwrap();

        let (vertex_buffer, slice) = factory.create_vertex_buffer_with_slice(&TRIANGLE, ());
        let (_, gui_texture) = load_texture_from_bytes(&mut factory, &[127; 4], 1, 1).unwrap();

        let gui_context = eframe::egui::CtxRef::default();

        Display {
            pso,
            pso_data: pipe::Data {
                u_stereo: 0,
                u_resolution_in: [1.0, 1.0],
                u_resolution_out: [1.0, 1.0],
                s_source: (src, sampler.clone()),
                s_deflection: (s_deflection, sampler.clone()),
                s_color_change:(s_color_change, sampler.clone()),
                s_color_uncertainty:(s_color_uncertainty, sampler.clone()),
                s_original:(s_original, sampler.clone()),
                s_covariances: (s_covariances, sampler.clone()),
                rt_color: dst.clone(),
                u_vis_type: 0,
                u_heat_scale: 1.0,
                u_dir_calc_scale: 1.0,
                u_flow_idx: 0,
                u_hive_rotation: [[0.0; 4]; 4],
                u_hive_position: [0.0; 2],
                u_hive_visible: 0,
                u_base_image: 0,
                u_combination_function: 0,
                u_mix_type: 0,
                u_colormap_type: 0
            },
            gui_pso,
            gui_pso_data: gui_pipe::Data {
                vbuf: vertex_buffer,
                u_texture: (gui_texture, sampler.clone()),
                u_resolution_in: [1.0, 1.0],
                u_resolution_out: [1.0, 1.0],
                rt_color: dst.clone(),
            },
            hive_rot: Matrix4::from_angle_x(Rad(0.0)),
            resolution: [1.0, 1.0],
            gui_context,
            gui_texture_version: 0,
            gui_meshes: Vec::new()
        }
    }

    fn negociate_slots(&mut self, window: &window::Window, slots: NodeSlots) -> NodeSlots {
        let slots = slots.to_color_input(window).to_color_output(window);
        self.pso_data.u_resolution_in = slots.input_size_f32();
        self.pso_data.u_resolution_out = slots.output_size_f32();
        self.pso_data.s_source = slots.as_color_view();
        self.pso_data.s_deflection = slots.as_deflection_view();
        self.pso_data.s_color_change = slots.as_color_change_view();
        self.pso_data.s_color_uncertainty = slots.as_color_uncertainty_view();
        self.pso_data.s_covariances = slots.as_covariances_view();
        
        self.pso_data.rt_color = slots.as_color();

        self.gui_pso_data.u_resolution_in = slots.input_size_f32();
        self.gui_pso_data.u_resolution_out = slots.output_size_f32();
        self.gui_pso_data.rt_color = slots.as_color();

        slots
    }

    fn negociate_slots_wk(&mut self, window: &window::Window, slots: NodeSlots, well_known: &WellKnownSlots) -> NodeSlots{
        self.pso_data.s_original = well_known.get_original().expect("Nah, no original image?");
        self.resolution = slots.output_size_f32();
        self.negociate_slots(window, slots)
    }


    fn update_values(&mut self, _window: &window::Window, values: &ValueMap) {
        self.pso_data.u_stereo = if values
            .get("split_screen_switch")
            .unwrap_or(&Value::Bool(false))
            .as_bool()
            .unwrap_or(false)
        {
            1
        } else {
            0
        };

        self.pso_data.u_flow_idx = values.get("flow_id").unwrap_or(&Value::Number(0.0)).as_f64().unwrap_or(0.0) as i32;
    }

    fn input(&mut self, perspective: &EyePerspective, vis_param: &VisualizationParameters) -> EyePerspective {
        //self.pso_data.u_vis_type = ((vis_param.vis_type) as u32) as i32;
        self.pso_data.u_heat_scale = vis_param.heat_scale;
        self.pso_data.u_dir_calc_scale = vis_param.dir_calc_scale;
        self.pso_data.u_flow_idx = vis_param.eye_idx as i32;
        self.pso_data.u_hive_position[0] = vis_param.highlight_position.0 as f32;
        self.pso_data.u_hive_position[1] = vis_param.highlight_position.1 as f32;
        self.pso_data.u_hive_visible = vis_param.bees_visible as i32;

        self.pso_data.u_base_image =  ((vis_param.vis_type.base_image) as u32) as i32;
        self.pso_data.u_combination_function = ((vis_param.vis_type.combination_function) as u32) as i32;
        self.pso_data.u_mix_type = ((vis_param.vis_type.mix_type) as u32) as i32;
        self.pso_data.u_colormap_type = ((vis_param.vis_type.color_map_type) as u32) as i32;

        let mut raw_input = eframe::egui::RawInput::default();
        raw_input.events.push(eframe::egui::Event::PointerButton{
            pos: eframe::egui::pos2(vis_param.mouse_input.position.0, vis_param.mouse_input.position.1),
            button: eframe::egui::PointerButton::Primary,
            pressed: vis_param.mouse_input.left_button,
            modifiers: eframe::egui::Modifiers::default(),
        });
        self.gui_context.begin_frame(raw_input);

        eframe::egui::Window::new("Window").show(&self.gui_context, |ui| {
            if ui.button("Click me").clicked() {
                println!("Click");
            }
        });

        let (output, shapes) = self.gui_context.end_frame();
        self.gui_meshes = self.gui_context.tessellate(shapes);

        perspective.clone()
    }

    fn render(&mut self, window: &window::Window) {

        let speed = 4.0;

        self.hive_rot= self.hive_rot*Matrix4::from_angle_x(Rad(       speed * window.delta_t()/1_000_000.0));
        self.hive_rot= self.hive_rot*Matrix4::from_angle_y(Rad( 0.7 * speed * window.delta_t()/1_000_000.0));
        self.hive_rot= self.hive_rot*Matrix4::from_angle_z(Rad( 0.2 * speed * window.delta_t()/1_000_000.0));

        self.pso_data.u_hive_rotation = self.hive_rot.into();

        let mut encoder = window.encoder().borrow_mut();

        if self.pso_data.u_stereo == 0 {
            encoder.draw(&gfx::Slice::from_vertex_count(6), &self.pso, &self.pso_data);
        } else {
            encoder.draw(
                &gfx::Slice::from_vertex_count(12),
                &self.pso,
                &self.pso_data,
            );
        }

        let mut factory = window.factory().borrow_mut();

        let gui_texture = self.gui_context.texture();
        if self.gui_texture_version != gui_texture.version{
            let mut tex_data: Vec<u8> = Vec::new();
            for color in gui_texture.srgba_pixels(){
                tex_data.push(color.r());
                tex_data.push(color.g());
                tex_data.push(color.b());
                tex_data.push(color.a());
            }
            let sampler = factory.create_sampler_linear();
            let (_, gui_texture_view) = load_texture_from_bytes(&mut factory, tex_data.as_slice(), gui_texture.width as u32, gui_texture.height as u32).unwrap();
            self.gui_pso_data.u_texture = (gui_texture_view, sampler);
            self.gui_texture_version = gui_texture.version;
        }

        /*for mesh in self.gui_meshes.iter(){
            let mut vertices = Vec::new();
            for v in &mesh.1.vertices{
                vertices.push(Vertex{ pos: [ v.pos.x, v.pos.y ],  uv: [ v.uv.x, v.uv.y ], color: [v.color.r() as f32 / 255.0, v.color.g() as f32 / 255.0, v.color.b() as f32 / 255.0, v.color.a() as f32 / 255.0] });
            }
            let (vertex_buffer, slice) = factory.create_vertex_buffer_with_slice(&vertices, mesh.1.indices.as_slice());
            self.gui_pso_data.vbuf = vertex_buffer;
            encoder.draw(&slice, &self.gui_pso, &self.gui_pso_data);
        }*/
    }
}
