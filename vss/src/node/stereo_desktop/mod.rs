use super::*;
use gfx;
use std::rc::Rc;
use std::cell::RefCell;

gfx_defines! {
    pipeline pipe {
        u_resolution_in: gfx::Global<[f32; 2]> = "u_resolution_in",
        u_resolution_out: gfx::Global<[f32; 2]> = "u_resolution_out",
        u_flow_idx: gfx::Global<i32> = "u_flow_idx",
        s_source_r: gfx::TextureSampler<[f32; 4]> = "s_color_r",
        s_source_l: gfx::TextureSampler<[f32; 4]> = "s_color_l",
        rt_color: gfx::RenderTarget<ColorFormat> = "rt_color",
    }
}

pub struct SharedStereoDesktop{
  idx_ctr: u32,
  shared: Rc<RefCell<SharedStereoDesktopData>>
}

pub struct SharedStereoDesktopData{
  s_source_r: Option<gfx::handle::ShaderResourceView<gfx_device_gl::Resources, [f32; 4]>>,
  s_source_l: Option<gfx::handle::ShaderResourceView<gfx_device_gl::Resources, [f32; 4]>>
}

impl SharedStereoDesktop{
  pub fn new() -> Self{
    SharedStereoDesktop{
      idx_ctr: 0,
      shared: Rc::new(RefCell::new(SharedStereoDesktopData{s_source_r: None, s_source_l: None}))
    }
  }
  pub fn get_stereo_desktop_node(&mut self,window: &Window,)->StereoDesktop{
    let desktop = StereoDesktop::new_from_shared(window, self.shared.clone(), self.idx_ctr);
    self.idx_ctr+=1;
    desktop
  }
}

pub struct StereoDesktop {
    pso: gfx::PipelineState<Resources, pipe::Meta>,
    pso_data: pipe::Data<Resources>,
    eye_idx: u32,
    shared: Option<Rc<RefCell<SharedStereoDesktopData>>>,    
}

impl StereoDesktop{
  fn new_from_shared(window: &Window, shared: Rc<RefCell<SharedStereoDesktopData>>, eye_idx: u32) -> Self {
    let mut proto = StereoDesktop::new(window);
    proto.shared = Some(shared);
    proto.eye_idx = eye_idx;
    proto
  }
}

impl Node for StereoDesktop {
    fn new(window: &Window) -> Self {
        let mut factory = window.factory().borrow_mut();

        let pso = factory
            .create_pipeline_simple(
              &include_glsl!("mod.vert"),
              &include_glsl!("mod.frag"),
                pipe::new(),
            )
            .unwrap();

        let sampler = factory.create_sampler_linear();
        let (_, src_r, dst) = factory.create_render_target(1, 1).unwrap();
        let (_, src_l, _): (
            _,
            _,
            gfx::handle::RenderTargetView<gfx_device_gl::Resources, [f32; 4]>,
        ) = factory.create_render_target(1, 1).unwrap();
        StereoDesktop {
            pso,
            pso_data: pipe::Data {
              u_resolution_in: [1.0, 1.0],
                u_resolution_out: [1.0, 1.0],
                u_flow_idx: 0,
                s_source_r: (src_r, sampler.clone()),
                s_source_l: (src_l, sampler),
                rt_color: dst,
            },
            eye_idx: 0,
            shared: None
        }
    }

    fn input(&mut self, perspective: &EyePerspective, vis_param: &VisualizationParameters) -> EyePerspective {
      self.pso_data.u_flow_idx = vis_param.eye_idx as i32;
      perspective.clone()
    }

    fn negociate_slots(&mut self, window: &Window, slots: NodeSlots) -> NodeSlots {
        let slots = slots.to_color_input(window).to_color_output(window);
        self.pso_data.u_resolution_in = slots.input_size_f32();
        self.pso_data.u_resolution_out = slots.output_size_f32();
        // self.pso_data.s_source_r = slots.as_color_view();
        self.pso_data.rt_color = slots.as_color();
        let cv = slots.as_color_view();
        match &self.shared{
          Some(shared) => {
            let mut guard = shared.borrow_mut();
            match self.eye_idx {
              0 => guard.s_source_r = Some(cv.0),
              1 => guard.s_source_l = Some(cv.0),
              _ => panic!("More than two eyes")
            }
            match &guard.s_source_r{
              Some(tex) => self.pso_data.s_source_r = (tex.clone(),cv.1.clone()),
              _ => {}
            }
            match &guard.s_source_l{
              Some(tex) => self.pso_data.s_source_l = (tex.clone(),cv.1.clone()),
              _ => {}
            }
          },
          None => {}
        }

        slots
    }

    fn render(&mut self, window: &Window) {
      if self.eye_idx == 1 {
        // println!("Draw sd");
        let mut encoder = window.encoder().borrow_mut();
        encoder.draw(&gfx::Slice::from_vertex_count(6), &self.pso, &self.pso_data);
      }
      else{
        // println!("Skip sd");
      }
    }
}
