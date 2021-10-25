#[derive(Copy, Clone, Debug)]
pub enum BaseImage{
  Output,
  Original,
  Ganglion,
  Variance
}

#[derive(Copy, Clone, Debug)]
pub enum CombinationFunction{
  AbsoluteErrorRGBVectorLength,
  AbsoluteErrorXYVectorLength,
  AbsoluteErrorRGBXYVectorLength,
  UncertaintyRGBVectorLength,
  UncertaintyXYVectorLength,
  UncertaintyRGBXYVectorLength,
  UncertaintyGenVar
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum MixType{
  BaseImageOnly,
  ColorMapOnly,
  OverlayThreshold,  
}

#[derive(Copy, Clone, Debug)]
pub enum ColorMapType{
  Viridis,
  Turbo,
  Grayscale,
}

impl Default for BaseImage{
  fn default() -> Self{
    BaseImage::Output
  }
}
impl Default for ColorMapType{
  fn default() -> Self{
    ColorMapType::Viridis
  }
}
impl Default for MixType{
  fn default() -> Self{
    MixType::BaseImageOnly
  }
}

impl Default for CombinationFunction{
  fn default() -> Self{
    CombinationFunction::AbsoluteErrorRGBVectorLength
  }
}

#[derive(Copy, Clone, Debug, Default)]
pub struct VisualizationType{
  pub base_image: BaseImage,
  pub combination_function: CombinationFunction,
  pub mix_type: MixType,
  pub color_map_type: ColorMapType
}

#[derive(Copy, Clone, Debug, Default)]
pub struct VisMouseInput{
  pub position: (f32,f32),
  pub left_button: bool,
  pub right_button: bool,
}

#[derive(Copy, Clone, Debug)]
pub struct VisualizationParameters{
  pub vis_type: VisualizationType,
  pub measure_variance: u32,
  pub variance_metric: u32,
  pub variance_color_space: u32,
  pub heat_scale : f32,
  pub dir_calc_scale : f32,
  pub test_depth_min : f32,
  pub test_depth_max : f32,
  pub astigmatism_strength: f32,
  pub eye_idx: u32,
  pub edit_eye_position: u32,
  pub eye_position: (f32,f32),
  pub previous_mouse_position: (f32,f32),
  pub highlight_position: (f64,f64),
  pub bees_flying: bool,
  pub bees_visible: bool,
  pub mouse_input: VisMouseInput,
}

impl Default for VisualizationParameters{
  fn default() -> Self{
    Self{
      vis_type: VisualizationType::default(),
      measure_variance: 0,
      variance_metric: 0,
      variance_color_space: 0,
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
      previous_mouse_position: (0.0,0.0),
      highlight_position: (0.0,0.0),
      bees_flying: true,
      bees_visible: false,
      mouse_input: VisMouseInput::default()
    }
  }
}

impl VisualizationParameters{
  pub fn has_to_track_error( &self) -> bool{
    // true
    self.vis_type.mix_type != MixType::BaseImageOnly
  }
}
