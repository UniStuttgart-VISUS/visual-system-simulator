use super::*;
use cgmath::Matrix4;
use cgmath::Rad;
use wgpu::CommandEncoder;

// #[repr(C, align(8))]
struct Uniforms{
    hive_rotation: [[f32; 4];4],

    resolution_in: [f32; 2],
    resolution_out: [f32; 2],
    hive_position: [f32; 2],

    hive_visible: i32,
    stereo: i32,
    flow_idx: i32,

    heat_scale: f32,
    dir_calc_scale: f32,

    base_image: i32,
    combination_function: i32,
    mix_type: i32,
    colormap_type: i32,
    
    _padding: u32, // fill up 16 byte pattern
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
//         u_resolution_in: gfx::Global<[f32; 2]> = "u_resolution_in",
//         u_resolution_out: gfx::Global<[f32; 2]> = "u_resolution_out",
//         rt_color: gfx::BlendTarget<ColorFormat> = ("rt_color", gfx::state::ColorMask::all(), gfx::preset::blend::ALPHA),
//     }
// }

// const TRIANGLE: [Vertex; 3] = [
//     Vertex { pos: [ -0.5, -0.5 ], uv: [ 0.0, 0.0 ], color: [1.0, 0.0, 0.0, 1.0] },
//     Vertex { pos: [  0.5, -0.5 ], uv: [ 0.0, 0.0 ], color: [0.0, 1.0, 0.0, 1.0] },
//     Vertex { pos: [  0.0,  0.5 ], uv: [ 0.0, 0.0 ], color: [0.0, 0.0, 1.0, 1.0] }
// ];

pub struct Display {
    hive_rot: Matrix4<f32>,
    // gui_context: eframe::egui::CtxRef,
    // gui_texture_version: u64,
    // gui_meshes: Vec<eframe::egui::ClippedMesh>,
    // gui_active: bool,
    pipeline: wgpu::RenderPipeline,
    uniforms: ShaderUniforms<Uniforms>,
    sources_bind_group: wgpu::BindGroup,
    original_bind_group: wgpu::BindGroup,
    render_target: RenderTexture,
}

impl Display {
    pub fn new(surface: &Surface) -> Self {
        let device = surface.device().borrow_mut();
        let queue = surface.queue().borrow_mut();

        let uniforms = ShaderUniforms::new(&device, 
            Uniforms{
                stereo: 0,
                resolution_in: [1.0, 1.0],
                resolution_out: [1.0, 1.0],
                flow_idx: 0,
            
                heat_scale: 1.0,
                dir_calc_scale: 1.0,
            
                hive_rotation: [[0.0; 4]; 4],
                hive_position: [0.0; 2],
                hive_visible: 0,
            
                base_image: 0,
                combination_function: 0,
                mix_type: 0,
                colormap_type: 0,
                _padding: 0,
            });
        
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
   

    fn negociate_slots(&mut self, surface: &Surface, slots: NodeSlots) -> NodeSlots {
        let slots = slots.to_color_input(surface).to_color_output(surface, "DisplayNode");
        let device = surface.device().borrow_mut();

        self.uniforms.data.resolution_in = slots.input_size_f32();
        self.uniforms.data.resolution_out = slots.output_size_f32();

        self.sources_bind_group = slots.as_all_colors_source(&device);
        self.render_target = slots.as_color_target();

        slots
    }

    fn negociate_slots_wk(&mut self, surface: &Surface, slots: NodeSlots, well_known: &WellKnownSlots) -> NodeSlots{
        match well_known.get_original() {
            Some(o) => {
                let device = surface.device().borrow_mut();
                self.original_bind_group = o.create_bind_group(&device).1
            },
            None => {},
        };
        self.negociate_slots(surface, slots)
    }


    fn update_values(&mut self, _surface: &Surface, values: &ValueMap) {
        self.uniforms.data.stereo = if values
            .get("split_screen_switch")
            .unwrap_or(&Value::Bool(false))
            .as_bool()
            .unwrap_or(false)
        {
            1
        } else {
            0
        };

        self.uniforms.data.flow_idx = values.get("flow_id").unwrap_or(&Value::Number(0.0)).as_f64().unwrap_or(0.0) as i32;
    }

    fn input(&mut self, perspective: &EyePerspective, vis_param: &VisualizationParameters) -> EyePerspective {
        self.uniforms.data.heat_scale = vis_param.heat_scale;
        self.uniforms.data.dir_calc_scale = vis_param.dir_calc_scale;
        self.uniforms.data.flow_idx = vis_param.eye_idx as i32;
        self.uniforms.data.hive_position[0] = vis_param.highlight_position.0 as f32;
        self.uniforms.data.hive_position[1] = vis_param.highlight_position.1 as f32;
        self.uniforms.data.hive_visible = vis_param.bees_visible as i32;

        self.uniforms.data.base_image =  vis_param.vis_type.base_image as i32;
        self.uniforms.data.combination_function = vis_param.vis_type.combination_function as i32;
        self.uniforms.data.mix_type = vis_param.vis_type.mix_type as i32;
        self.uniforms.data.colormap_type = vis_param.vis_type.color_map_type as i32;

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
        let speed = 4.0;

        self.hive_rot= self.hive_rot*Matrix4::from_angle_x(Rad(       speed * surface.delta_t()/1_000_000.0));
        self.hive_rot= self.hive_rot*Matrix4::from_angle_y(Rad( 0.7 * speed * surface.delta_t()/1_000_000.0));
        self.hive_rot= self.hive_rot*Matrix4::from_angle_z(Rad( 0.2 * speed * surface.delta_t()/1_000_000.0));

        self.uniforms.data.hive_rotation = self.hive_rot.into();

        self.uniforms.update(&surface.queue().borrow_mut());

        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("DisplayNode render_pass"),
            color_attachments: &[screen.unwrap_or(&self.render_target).to_color_attachment()],
            depth_stencil_attachment: None,
        });
    
        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, &self.uniforms.bind_group, &[]);
        render_pass.set_bind_group(1, &self.sources_bind_group, &[]);
        render_pass.set_bind_group(2, &self.original_bind_group, &[]);
        if self.uniforms.data.stereo == 0 {
            render_pass.draw(0..6, 0..1);
        }else{
            render_pass.draw(0..12, 0..1);
        }

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
