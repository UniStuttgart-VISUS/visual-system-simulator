use super::*;

use cgmath::Rad;
use std::ops::Mul;

/// A node that implements eye control.
pub struct EyeControl {
    configured_view: Matrix4<f32>,

    eye_axis_rot_x: f64,
    eye_axis_rot_y: f64,
}

impl EyeControl {
    pub fn new(_surface: &Surface) -> Self {
        EyeControl {
            configured_view: Matrix4::from_scale(1.0),
            eye_axis_rot_x: 0.0,
            eye_axis_rot_y: 0.0,
        }
    }
}

impl Node for EyeControl {
    fn negociate_slots(
        &mut self,
        _surface: &Surface,
        slots: NodeSlots,
        _original_image: &mut Option<Texture>,
    ) -> NodeSlots {
        slots.to_passthrough()
    }

    fn inspect(&mut self, inspector: &mut dyn Inspector) {
        inspector.begin_node("EyeControl");

        let mut configured_view = Matrix4::from_scale(1.0);

        // if the eye has strabism, it needs some angle offset
        inspector.mut_f64("eye_axis_rot_x", &mut self.eye_axis_rot_x);
        configured_view =
            configured_view.mul(Matrix4::from_angle_x(Rad(self.eye_axis_rot_x as f32)));

        inspector.mut_f64("eye_axis_rot_y", &mut self.eye_axis_rot_y);
        configured_view =
            configured_view.mul(Matrix4::from_angle_y(Rad(self.eye_axis_rot_y as f32)));

        self.configured_view = configured_view;

        inspector.end_node();
    }

    fn input(
        &mut self,
        perspective: &EyePerspective,
        _vis_param: &VisualizationParameters,
    ) -> EyePerspective {
        // let mut vp = self.surface.vis_param.borrow_mut();
        // vp.mouse_input.position = (position.x as f32, position.y as f32);
        // match vp.edit_eye_position {
        //     1 => {
        //         vp.previous_mouse_position = (position.x as f32 * 0.1, position.y as f32 * 0.1);
        //         vp.edit_eye_position = 2;
        //     }
        //     2 => {
        //         let (p_x, p_y) = vp.previous_mouse_position;
        //         let (c_x, c_y) = (position.x as f32 * 0.1, position.y as f32 * 0.1);
        //         vp.eye_position = (c_x - p_x, c_y - p_y);
        //     }
        //     _ => {}
        // }

        let mut perspective = perspective.clone();
        perspective.view = self.configured_view.mul(perspective.view);
        perspective
    }

    fn render(
        &mut self,
        _surface: &Surface,
        _encoder: &mut CommandEncoder,
        _screen: Option<&RenderTexture>,
    ) {
    }
}
