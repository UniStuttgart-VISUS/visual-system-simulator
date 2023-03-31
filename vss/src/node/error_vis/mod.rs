use std::borrow::BorrowMut;

use super::*;
use cgmath::Matrix4;
use cgmath::Rad;
use wgpu::CommandEncoder;

struct Uniforms{
    hive_rotation: [[f32; 4];4],

    resolution_in: [f32; 2],
    hive_position: [f32; 2],

    hive_visible: i32,
    flow_idx: i32,

    heat_scale: f32,
    dir_calc_scale: f32,

    base_image: i32,
    combination_function: i32,
    mix_type: i32,
    colormap_type: i32,
}

pub struct ErrorVis {
    hive_rot: Matrix4<f32>,
    pipeline: wgpu::RenderPipeline,
    uniforms: ShaderUniforms<Uniforms>,
    sources_bind_group: wgpu::BindGroup,
    original_bind_group: wgpu::BindGroup,
    render_target: RenderTexture,
}

impl ErrorVis {
    pub fn new(surface: &Surface) -> Self {
        let device = surface.device().borrow_mut();
        let queue = surface.queue().borrow_mut();

        let uniforms = ShaderUniforms::new(&device, 
            Uniforms{
                resolution_in: [1.0, 1.0],
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
            });
        
        let (sources_bind_group_layout, sources_bind_group) = create_color_sources_bind_group(&device, &queue, "ErrorVisNode");

        let original_tex = placeholder_texture(&device, &queue, Some("ErrorVisNode s_original")).unwrap();
        let(original_bind_group_layout, original_bind_group) = original_tex.create_bind_group(&device);
        
        let render_target = placeholder_color_rt(&device, Some("ErrorVisNode render_target"));

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("ErrorVisNode Shader"),
            source: wgpu::ShaderSource::Wgsl(concat!(
                include_str!("../common.wgsl"),
                include_str!("../vert.wgsl"),
                include_str!("mod.wgsl")).into()),
        });

        let pipeline = create_render_pipeline(
            &device,
            &[&shader, &shader],
            &["vs_main", "fs_main"],
            &[&uniforms.bind_group_layout, &sources_bind_group_layout, &original_bind_group_layout],
            &[blended_color_state(COLOR_FORMAT)],
            None,
            Some("ErrorVisNode Render Pipeline")
        );

        ErrorVis {
            hive_rot: Matrix4::from_angle_x(Rad(0.0)),
            pipeline,
            uniforms,
            sources_bind_group,
            original_bind_group,
            render_target,
        }
    }
}

impl Node for ErrorVis {
   

    fn negociate_slots(&mut self, surface: &Surface, slots: NodeSlots, original_image: &mut Option<Texture>) -> NodeSlots {
        let slots = slots.to_color_input(surface).to_color_output(surface, "ErrorVisNode");
        let device = surface.device().borrow_mut();

        self.uniforms.data.resolution_in = slots.input_size_f32();

        self.sources_bind_group = slots.as_all_colors_source(&device);
        self.render_target = slots.as_color_target();
        if let Some(tex) = original_image.borrow_mut() {
            (_, self.original_bind_group) = tex.create_bind_group(&device);
        }

        slots
    }

    fn update_values(&mut self, _surface: &Surface, values: &ValueMap) {
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
            label: Some("ErrorVisNode render_pass"),
            color_attachments: &[screen.unwrap_or(&self.render_target).to_color_attachment(Some(CLEAR_COLOR))],
            depth_stencil_attachment: None,
        });
    
        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, &self.uniforms.bind_group, &[]);
        render_pass.set_bind_group(1, &self.sources_bind_group, &[]);
        render_pass.set_bind_group(2, &self.original_bind_group, &[]);
        render_pass.draw(0..6, 0..1);
    }
}
