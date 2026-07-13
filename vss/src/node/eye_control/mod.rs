use super::*;

use cgmath::Rad;
use std::ops::Mul;
use std::sync::OnceLock;

/// A node that implements eye control.
pub struct EyeControl {
    config: EyeControlConfig,
    configured_view: Matrix4<f32>,
    edit_eye_position: u32,
}

pub struct EyeControlConfig {
    eye_axis_rot_x: f64,
    eye_axis_rot_y: f64,
}

impl Default for EyeControlConfig {
    fn default() -> Self {
        Self {
            eye_axis_rot_x: 0.0,
            eye_axis_rot_y: 0.0,
        }
    }
}

pub const AXIS_X: ParameterId<EyeControl, f64> =
    ParameterId::for_node("eye.axis-x", |n| &mut n.config.eye_axis_rot_x);
pub const AXIS_Y: ParameterId<EyeControl, f64> =
    ParameterId::for_node("eye.axis-y", |n| &mut n.config.eye_axis_rot_y);
impl Parameters for EyeControl {
    fn parameters() -> &'static [ParameterDescriptor] {
        static P: OnceLock<Vec<ParameterDescriptor>> = OnceLock::new();
        P.get_or_init(|| vec![AXIS_X.descriptor(), AXIS_Y.descriptor()])
    }
}

impl EyeControl {
    pub fn new(_context: &RenderContext) -> Self {
        EyeControl {
            config: EyeControlConfig::default(),
            configured_view: Matrix4::from_scale(1.0),
            edit_eye_position: 0,
        }
    }
}

impl Node for EyeControl {
    fn name(&self) -> &'static str {
        "EyeControl"
    }

    fn negociate_slots(
        &mut self,
        _context: &RenderContext,
        slots: NodeSlots,
        _original_image: &mut Option<Texture>,
    ) -> NodeSlots {
        slots.to_passthrough()
    }

    fn configure(&mut self) -> NodeChanges {
        let configured_view = Matrix4::from_angle_x(Rad(self.config.eye_axis_rot_x as f32)).mul(
            Matrix4::from_angle_y(Rad(self.config.eye_axis_rot_y as f32)),
        );
        let output_changed = self.configured_view != configured_view;
        self.configured_view = configured_view;
        NodeChanges::from_output_slots(output_changed, false)
    }

    fn input(&mut self, eye: &EyeInput, _mouse: &MouseInput) -> (EyeInput, NodeChanges) {
        // vp.mouse_input.position = (position.x as f32, position.y as f32);
        match self.edit_eye_position {
            1 => {
                //         vp.previous_mouse_position = (position.x as f32 * 0.1, position.y as f32 * 0.1);
                self.edit_eye_position = 2;
            }
            //     2 => {
            //         let (p_x, p_y) = vp.previous_mouse_position;
            //         let (c_x, c_y) = (position.x as f32 * 0.1, position.y as f32 * 0.1);
            //         vp.eye_position = (c_x - p_x, c_y - p_y);
            //     }
            _ => {}
        }

        let mut eye = eye.clone();
        eye.view = self.configured_view.mul(eye.view);
        (eye, NodeChanges::empty())
    }

    fn render(
        &mut self,
        _context: &RenderContext,
        _encoder: &mut CommandEncoder,
        _screen: Option<&RenderTexture>,
    ) {
    }
}
