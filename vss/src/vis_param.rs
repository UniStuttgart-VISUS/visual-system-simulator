#[derive(Copy, Clone)]
pub enum VisualizationType{
  Output,
  Deflection
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
  pub dir_calc_scale : f32
}

impl Default for VisualizationParameters{
  fn default() -> Self{
    Self{
      vis_type: VisualizationType::default(),
      heat_scale: 1.0,
      dir_calc_scale: 1.0
    }
  }
}