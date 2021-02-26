#[derive(Copy, Clone)]
pub enum VisualizationType{
  Output,
  Deflection,
  ColorChange,
  ColorUncertainty,
  Original,
  OverlayOutput,
  OverlayInput,
  Ganglion
}

impl Default for VisualizationType{
  fn default() -> Self{
    VisualizationType::Output
  }
}

#[derive(Copy, Clone)]
pub struct VisualizationParameters{
  pub vis_type: VisualizationType,
  pub heat_scale : f32,
  pub dir_calc_scale : f32,
  pub test_depth_min : f32,
  pub test_depth_max : f32,
  pub astigmatism_strength: f32,
  pub eye_idx: u32,
  pub edit_eye_position: u32,
  pub eye_position: (f32,f32),
  pub previous_mouse_position: (f32,f32)
}

impl Default for VisualizationParameters{
  fn default() -> Self{
    Self{
      vis_type: VisualizationType::default(),
      heat_scale: 1.0,
      dir_calc_scale: 0.0,
      // test_depth_min: 100.0,
      // test_depth_max: 300.0,
      test_depth_min: 200.0,
      test_depth_max: 5000.0,
      astigmatism_strength: 0.0,
      eye_idx: 0,
      edit_eye_position: 0,
      eye_position: (0.0,0.0),
      previous_mouse_position: (0.0,0.0)

    }
  }
}