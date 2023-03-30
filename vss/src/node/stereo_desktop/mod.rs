use super::*;
use std::cell::RefCell;
use std::rc::Rc;
struct Uniforms {
    resolution_in: [f32; 2],
    resolution_out: [f32; 2],
    flow_idx: i32,
    _padding: u32,
}
pub struct SharedStereoDesktop {
    idx_ctr: u32,
    shared: Rc<RefCell<SharedStereoDesktopData>>,
}

pub struct SharedStereoDesktopData {
    s_source_r: Option<Texture>,
    s_source_l: Option<Texture>,
}

impl SharedStereoDesktop {
    pub fn new() -> Self {
        SharedStereoDesktop {
            idx_ctr: 0,
            shared: Rc::new(RefCell::new(SharedStereoDesktopData {
                s_source_r: None,
                s_source_l: None,
            })),
        }
    }
    pub fn get_stereo_desktop_node(&mut self, surface: &Surface) -> StereoDesktop {
        let desktop = StereoDesktop::new_from_shared(surface, self.shared.clone(), self.idx_ctr);
        self.idx_ctr += 1;
        desktop
    }
}

pub struct StereoDesktop {
    pipeline: wgpu::RenderPipeline,
    uniforms: ShaderUniforms<Uniforms>,
    source_r: wgpu::BindGroup,
    source_l: wgpu::BindGroup,
    target: RenderTexture,

    eye_idx: u32,
    shared: Option<Rc<RefCell<SharedStereoDesktopData>>>,
}

impl StereoDesktop {
    fn new(surface: &Surface) -> Self {
        let device = surface.device().borrow_mut();
        let queue = surface.queue().borrow_mut();

        let uniforms = ShaderUniforms::new(
            &device,
            Uniforms {
                resolution_in: [1.0, 1.0],
                resolution_out: [1.0, 1.0],
                flow_idx: 0,
                _padding: 0,
            },
        );

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Stereo Shader"),
            source: wgpu::ShaderSource::Wgsl(concat!(include_str!("mod.wgsl")).into()),
        });

        let (source_r_bind_group_layout, source_r_bind_group) = placeholder_texture(
            &device,
            &queue,
            Some("Stereo Shader source_r (placeholder)"),
        )
        .unwrap()
        .create_bind_group(&device);

        let (source_l_bind_group_layout, source_l_bind_group) = placeholder_texture(
            &device,
            &queue,
            Some("Stereo Shader source_l (placeholder)"),
        )
        .unwrap()
        .create_bind_group(&device);

        let target = placeholder_color_rt(&device, Some("Stereo target (placeholder)"));

        let pipeline = create_render_pipeline(
            &device,
            &[&shader, &shader],
            &["vs_main", "fs_main"],
            &[&uniforms.bind_group_layout, &source_r_bind_group_layout, &source_l_bind_group_layout],
            &[simple_color_state(COLOR_FORMAT)],
            None,
            Some("Stereo Render Pipeline"));

        StereoDesktop {
            pipeline,
            uniforms,
            source_r: source_r_bind_group,
            source_l: source_l_bind_group,
            target,
            eye_idx: 0,
            shared: None,
        }
    }
    
    fn new_from_shared(
        surface: &Surface,
        shared: Rc<RefCell<SharedStereoDesktopData>>,
        eye_idx: u32,
    ) -> Self {
        let mut proto = StereoDesktop::new(surface);
        proto.shared = Some(shared);
        proto.eye_idx = eye_idx;
        proto
    }
}

impl Node for StereoDesktop {
  

    fn input(
        &mut self,
        perspective: &EyePerspective,
        vis_param: &VisualizationParameters,
    ) -> EyePerspective {
        self.uniforms.data.flow_idx = vis_param.eye_idx as i32;
        perspective.clone()
    }

    fn negociate_slots(&mut self, surface: &Surface, slots: NodeSlots, _resolution: Option<[u32;2]>, _original_image: &mut Option<Texture>) -> NodeSlots {
        let slots = slots.to_color_input(surface).to_color_output(surface, "StereoNode");
        let device = surface.device().borrow_mut();

        self.uniforms.data.resolution_in = slots.input_size_f32();
        self.uniforms.data.resolution_out = slots.output_size_f32();
        // self.pso_data.s_source_r = slots.as_color_view();
        self.target = slots.as_color_target();
        let (cv, _) = slots.as_color_source(&device);
        match &self.shared {
            Some(shared) => {
                let mut guard = shared.borrow_mut();
                match self.eye_idx {
                    0 => guard.s_source_r = Some(cv),
                    1 => guard.s_source_l = Some(cv),
                    _ => panic!("More than two eyes"),
                }
                match &guard.s_source_r {
                    Some(tex) => (_, self.source_r) = tex.create_bind_group(&device),
                    _ => {}
                }
                match &guard.s_source_l {
                    Some(tex) => (_, self.source_l) = tex.create_bind_group(&device),
                    _ => {}
                }
            }
            None => {}
        }

        slots
    }

    fn render(&mut self, surface: &Surface, encoder: &mut CommandEncoder, screen: Option<&RenderTexture>) {
        if self.eye_idx == 1 {
            // println!("Draw sd");
            self.uniforms.update(&surface.queue().borrow_mut());
        
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Stereo render_pass"),
                color_attachments: &[screen.unwrap_or(&self.target).to_color_attachment()],
                depth_stencil_attachment: None,
            });
        
            render_pass.set_pipeline(&self.pipeline);
            render_pass.set_bind_group(0, &self.uniforms.bind_group, &[]);
            render_pass.set_bind_group(1, &self.source_r, &[]);
            render_pass.set_bind_group(2, &self.source_l, &[]);
            render_pass.draw(0..6, 0..1);
        } else {
            // println!("Skip sd");
        }
    }
}
