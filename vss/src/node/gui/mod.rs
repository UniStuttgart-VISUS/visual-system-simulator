// TODO everything here

use std::borrow::BorrowMut;

use super::*;
use cgmath::Matrix4;
use cgmath::Rad;
use wgpu::CommandEncoder;

struct Uniforms{
    resolution_in: [f32; 2],
    resolution_out: [f32; 2],
}

// gfx_defines! {
//     vertex Vertex {
//         pos: [f32; 2] = "a_pos",
//         uv: [f32; 2] = "a_uv",
//         color: [f32; 4] = "a_col",
//     }

//     pipeline gui_pipe {
//         vbuf: gfx::VertexBuffer<Vertex> = (),
//         u_texture: gfx::TextureSampler<[f32; 4]> = "u_tex",
//         rt_color: gfx::BlendTarget<ColorFormat> = ("rt_color", gfx::state::ColorMask::all(), gfx::preset::blend::ALPHA),
//     }
// }

// const TRIANGLE: [Vertex; 3] = [
//     Vertex { pos: [ -0.5, -0.5 ], uv: [ 0.0, 0.0 ], color: [1.0, 0.0, 0.0, 1.0] },
//     Vertex { pos: [  0.5, -0.5 ], uv: [ 0.0, 0.0 ], color: [0.0, 1.0, 0.0, 1.0] },
//     Vertex { pos: [  0.0,  0.5 ], uv: [ 0.0, 0.0 ], color: [0.0, 0.0, 1.0, 1.0] }
// ];

pub struct GUI {
    pipeline: wgpu::RenderPipeline,
    uniforms: ShaderUniforms<Uniforms>,
    sources_bind_group: wgpu::BindGroup,
    render_target: RenderTexture,

    gui_context: eframe::egui::CtxRef,
    gui_texture_version: u64,
    gui_meshes: Vec<eframe::egui::ClippedMesh>,
    gui_active: bool,
}

impl GUI {
    pub fn new(surface: &Surface) -> Self {
        let device = surface.device().borrow_mut();
        let queue = surface.queue().borrow_mut();
        
        let (sources_bind_group_layout, sources_bind_group) = create_color_sources_bind_group(&device, &queue, "DisplayNode");

        let original_tex = placeholder_texture(&device, &queue, Some("DisplayNode s_original")).unwrap();
        let(original_bind_group_layout, original_bind_group) = original_tex.create_bind_group(&device);
        
        let render_target = placeholder_color_rt(&device, Some("DisplayNode render_target"));

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("DisplayNode Shader"),
            source: wgpu::ShaderSource::Wgsl(concat!(
                include_str!("../common.wgsl"),
                include_str!("mod.wgsl")).into()),
        });

        let pipeline = create_render_pipeline(
            &device,
            &[&shader, &shader],
            &["vs_main", "fs_main"],
            &[&uniforms.bind_group_layout, &sources_bind_group_layout, &original_bind_group_layout],
            &[blended_color_state(COLOR_FORMAT)],
            None,
            Some("DisplayNode Render Pipeline")
        );
                
        // let gui_pso = factory
        //     .create_pipeline_simple(
        //         &include_glsl!("gui.vert"),
        //         &include_glsl!("gui.frag"),
        //         gui_pipe::new(),
        //     )
        //     .unwrap();

        // let (vertex_buffer, _slice) = factory.create_vertex_buffer_with_slice(&TRIANGLE, ());
        // let (_, gui_texture) = load_texture_from_bytes(&mut factory, &[127; 4], 1, 1).unwrap();

        // let gui_context = eframe::egui::CtxRef::default();

        Display {
            // gui_pso,
            // gui_pso_data: gui_pipe::Data {
            //     vbuf: vertex_buffer,
            //     u_texture: (gui_texture, sampler.clone()),
            //     u_resolution_in: [1.0, 1.0],
            //     u_resolution_out: [1.0, 1.0],
            //     rt_color: dst.clone(),
            // },
            hive_rot: Matrix4::from_angle_x(Rad(0.0)),
            // gui_context,
            // gui_texture_version: 0,
            // gui_meshes: Vec::new(),
            // gui_active: false
            pipeline,
            uniforms,
            sources_bind_group,
            original_bind_group,
            render_target,
        }
    }
}

impl Node for Display {
   

    fn negociate_slots(&mut self, surface: &Surface, slots: NodeSlots, original_image: &mut Option<Texture>) -> NodeSlots {
        let slots = slots.to_color_input(surface).to_color_output(surface, "DisplayNode");
        let device = surface.device().borrow_mut();

        self.uniforms.data.resolution_in = slots.input_size_f32();
        self.uniforms.data.resolution_out = slots.output_size_f32();

        self.sources_bind_group = slots.as_all_colors_source(&device);
        self.render_target = slots.as_color_target();
        if let Some(tex) = original_image.borrow_mut() {
            (_, self.original_bind_group) = tex.create_bind_group(&device);
        }

        slots
    }

    fn update_values(&mut self, _surface: &Surface, _values: &ValueMap) {

    }

    fn input(&mut self, perspective: &EyePerspective, vis_param: &VisualizationParameters) -> EyePerspective {
        // let mut raw_input = eframe::egui::RawInput::default();
        // raw_input.events.push(eframe::egui::Event::PointerButton{
        //     pos: eframe::egui::pos2(vis_param.mouse_input.position.0, vis_param.mouse_input.position.1),
        //     button: eframe::egui::PointerButton::Primary,
        //     pressed: vis_param.mouse_input.left_button,
        //     modifiers: eframe::egui::Modifiers::default(),
        // });
        // self.gui_context.begin_frame(raw_input);

        // eframe::egui::Window::new("Window").show(&self.gui_context, |ui| {
        //     if ui.button("Click me").clicked() {
        //         println!("Click");
        //     }
        // });

        // let (_output, shapes) = self.gui_context.end_frame();
        // self.gui_meshes = self.gui_context.tessellate(shapes);

        perspective.clone()
    }

    fn render(&mut self, surface: &Surface, encoder: &mut CommandEncoder, screen: Option<&RenderTexture>) {
        // if self.gui_active{
        //     let mut factory = window.factory().borrow_mut();
    
        //     let gui_texture = self.gui_context.texture();
        //     if self.gui_texture_version != gui_texture.version{
        //         let mut tex_data: Vec<u8> = Vec::new();
        //         for color in gui_texture.srgba_pixels(){
        //             tex_data.push(color.r());
        //             tex_data.push(color.g());
        //             tex_data.push(color.b());
        //             tex_data.push(color.a());
        //         }
        //         let sampler = factory.create_sampler_linear();
        //         let (_, gui_texture_view) = load_texture_from_bytes(&mut factory, tex_data.as_slice(), gui_texture.width as u32, gui_texture.height as u32).unwrap();
        //         self.gui_pso_data.u_texture = (gui_texture_view, sampler);
        //         self.gui_texture_version = gui_texture.version;
        //     }
    
        //     for mesh in self.gui_meshes.iter(){
        //         let mut vertices = Vec::new();
        //         for v in &mesh.1.vertices{
        //             vertices.push(Vertex{ pos: [ v.pos.x, v.pos.y ],  uv: [ v.uv.x, v.uv.y ], color: [v.color.r() as f32 / 255.0, v.color.g() as f32 / 255.0, v.color.b() as f32 / 255.0, v.color.a() as f32 / 255.0] });
        //         }
        //         let (vertex_buffer, slice) = factory.create_vertex_buffer_with_slice(&vertices, mesh.1.indices.as_slice());
        //         self.gui_pso_data.vbuf = vertex_buffer;
        //         encoder.draw(&slice, &self.gui_pso, &self.gui_pso_data);
        //     }
        // }
    }
}
