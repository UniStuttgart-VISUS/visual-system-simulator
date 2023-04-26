#[derive(Copy, Clone, Debug, Default)]
pub struct VisMouseInput {
    pub position: (f32, f32),
    pub left_button: bool,
    pub right_button: bool,
}

#[derive(Copy, Clone, Debug)]
pub struct VisualizationParameters {
    pub dir_calc_scale: f32,
    pub test_depth_min: f32,
    pub test_depth_max: f32,
    pub eye_position: (f32, f32),
    pub mouse_input: VisMouseInput,
}

impl Default for VisualizationParameters {
    fn default() -> Self {
        Self {
            dir_calc_scale: 0.0,
            test_depth_min: 200.0,
            test_depth_max: 5000.0,
            eye_position: (0.0, 0.0),
            mouse_input: VisMouseInput::default(),
        }
    }
}
 