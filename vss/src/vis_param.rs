use crate::Inspector;

#[derive(Copy, Clone, Debug)]
pub enum BaseImage {
    Output,
    Original,
    Ganglion,
    Variance,
}

#[derive(Copy, Clone, Debug)]
pub enum CombinationFunction {
    AbsoluteErrorRGBVectorLength,
    AbsoluteErrorXYVectorLength,
    AbsoluteErrorRGBXYVectorLength,
    UncertaintyRGBVectorLength,
    UncertaintyXYVectorLength,
    UncertaintyRGBXYVectorLength,
    UncertaintyGenVar,
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum MixType {
    BaseImageOnly,
    ColorMapOnly,
    OverlayThreshold,
}

#[derive(Copy, Clone, Debug)]
pub enum ColorMapType {
    Viridis,
    Turbo,
    Grayscale,
}

impl Default for BaseImage {
    fn default() -> Self {
        BaseImage::Output
    }
}
impl Default for ColorMapType {
    fn default() -> Self {
        ColorMapType::Viridis
    }
}
impl Default for MixType {
    fn default() -> Self {
        MixType::BaseImageOnly
    }
}

impl Default for CombinationFunction {
    fn default() -> Self {
        CombinationFunction::AbsoluteErrorRGBVectorLength
    }
}

#[derive(Copy, Clone, Debug, Default)]
pub struct VisualizationType {
    pub base_image: BaseImage,
    pub combination_function: CombinationFunction,
    pub mix_type: MixType,
    pub color_map_type: ColorMapType,
}

#[derive(Copy, Clone, Debug, Default)]
pub struct VisMouseInput {
    pub position: (f32, f32),
    pub left_button: bool,
    pub right_button: bool,
}

#[derive(Copy, Clone, Debug)]
pub struct VisualizationParameters {
    pub vis_type: VisualizationType,
    pub measure_variance: u32,
    pub variance_metric: u32,
    pub variance_color_space: u32,
    pub heat_scale: f32,
    pub dir_calc_scale: f32,
    pub test_depth_min: f32,
    pub test_depth_max: f32,
    pub astigmatism_strength: f32,
    pub eye_idx: u32,
    pub edit_eye_position: u32,
    pub eye_position: (f32, f32),
    pub previous_mouse_position: (f32, f32),
    pub highlight_position: (f32, f32),
    pub bees_flying: bool,
    pub bees_visible: bool,
    pub mouse_input: VisMouseInput,
}

impl Default for VisualizationParameters {
    fn default() -> Self {
        Self {
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
            eye_position: (0.0, 0.0),
            previous_mouse_position: (0.0, 0.0),
            highlight_position: (0.0, 0.0),
            bees_flying: true,
            bees_visible: false,
            mouse_input: VisMouseInput::default(),
        }
    }
}

impl VisualizationParameters {
    pub fn has_to_track_error(&self) -> bool {
        self.vis_type.mix_type != MixType::BaseImageOnly
    }

    pub fn inspect(&mut self, inspector: &mut dyn Inspector) {
        inspector.begin_node("VisualizationParameters");

        let mut file_base_image = self.vis_type.base_image as i32;
        if inspector.mut_i32("file_base_image", &mut file_base_image) {
            self.vis_type.base_image = match file_base_image {
                0 => BaseImage::Output,
                1 => BaseImage::Original,
                2 => BaseImage::Ganglion,
                _ => panic!("No BaseImage of {} found", file_base_image),
            };
        }

        let mut file_mix_type = self.vis_type.mix_type as i32;
        if inspector.mut_i32("file_mix_type", &mut file_mix_type) {
            self.vis_type.mix_type = match file_mix_type {
                0 => MixType::BaseImageOnly,
                1 => MixType::ColorMapOnly,
                2 => MixType::OverlayThreshold,
                _ => panic!("No MixType of {} found", file_mix_type),
            };
        }

        let mut file_cm_type = self.vis_type.color_map_type as i32;
        if inspector.mut_i32("file_cm_type", &mut file_cm_type) {
            self.vis_type.color_map_type = match file_cm_type {
                0 => ColorMapType::Viridis,
                1 => ColorMapType::Turbo,
                2 => ColorMapType::Grayscale,
                _ => panic!("No ColorMapType of {} found", file_cm_type),
            };
        }

        let mut file_cf = self.vis_type.combination_function as i32;
        if inspector.mut_i32("file_cf", &mut file_cf) {
            self.vis_type.combination_function = match file_cf {
                0 => CombinationFunction::AbsoluteErrorRGBVectorLength,
                1 => CombinationFunction::AbsoluteErrorXYVectorLength,
                2 => CombinationFunction::AbsoluteErrorRGBXYVectorLength,
                3 => CombinationFunction::UncertaintyRGBVectorLength,
                4 => CombinationFunction::UncertaintyXYVectorLength,
                5 => CombinationFunction::UncertaintyRGBXYVectorLength,
                6 => CombinationFunction::UncertaintyGenVar,
                _ => panic!("No CombinationFunction of {} found", file_cf),
            };
        }
        inspector.mut_f32("cm_scale", &mut self.heat_scale);
        inspector.mut_u32("measure_variance", &mut self.measure_variance);
        inspector.mut_u32("variance_metric", &mut self.variance_metric);
        inspector.mut_u32("variance_color_space", &mut self.variance_color_space);

        inspector.end_node();
    }
}
