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
  pub astigmatism_strength: f32

}

impl Default for VisualizationParameters{
  fn default() -> Self{
    Self{
      vis_type: VisualizationType::default(),
      heat_scale: 1.0,
      dir_calc_scale: 1.0,
      test_depth_min: 100.0,
      test_depth_max: 300.0,
      astigmatism_strength: 0.0
    }
  }
}