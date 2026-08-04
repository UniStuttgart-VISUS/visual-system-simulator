use crate::Flow;
use cgmath::{Matrix4, Rad, Vector3, Vector4};

#[derive(Clone, Copy, Debug)]
pub enum SemanticInput {
    GazeDelta([f32; 2]),
    ViewDelta([f32; 2]),
    ResetPose,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct IntentionalPose {
    gaze_yaw: f32,
    gaze_pitch: f32,
    view_yaw: f32,
    view_pitch: f32,
}

impl IntentionalPose {
    pub fn apply(&mut self, input: SemanticInput) {
        match input {
            SemanticInput::GazeDelta(delta) => {
                apply_pose_delta(&mut self.gaze_yaw, &mut self.gaze_pitch, delta)
            }
            SemanticInput::ViewDelta(delta) => {
                apply_pose_delta(&mut self.view_yaw, &mut self.view_pitch, delta)
            }
            SemanticInput::ResetPose => *self = Self::default(),
        }
    }

    pub fn view_matrix(self, base: Matrix4<f32>) -> Matrix4<f32> {
        Matrix4::from_angle_x(Rad(self.view_pitch))
            * Matrix4::from_angle_y(Rad(self.view_yaw))
            * base
    }

    pub fn gaze_vector(self, base: Vector3<f32>) -> Vector3<f32> {
        (Matrix4::from_angle_x(Rad(self.gaze_pitch))
            * Matrix4::from_angle_y(Rad(self.gaze_yaw))
            * Vector4::new(base.x, base.y, base.z, 0.0))
        .truncate()
    }

    pub fn apply_to_flows(self, flows: &[Flow]) {
        for flow in flows {
            let mut eye = flow.eye_mut();
            eye.view = self.view_matrix(Matrix4::from_scale(1.0));
            eye.gaze = self.gaze_vector(Vector3::unit_z());
        }
    }

    pub fn gaze_yaw_degrees(self) -> f32 {
        self.gaze_yaw.to_degrees()
    }

    pub fn gaze_pitch_degrees(self) -> f32 {
        self.gaze_pitch.to_degrees()
    }

    pub fn view_yaw_degrees(self) -> f32 {
        self.view_yaw.to_degrees()
    }

    pub fn view_pitch_degrees(self) -> f32 {
        self.view_pitch.to_degrees()
    }
}

fn apply_pose_delta(yaw: &mut f32, pitch: &mut f32, delta: [f32; 2]) {
    *yaw = (*yaw + delta[0] * std::f32::consts::PI + std::f32::consts::PI)
        .rem_euclid(std::f32::consts::TAU)
        - std::f32::consts::PI;
    let limit = 89.0_f32.to_radians();
    *pitch = (*pitch - delta[1] * std::f32::consts::FRAC_PI_2).clamp(-limit, limit);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalized_deltas_wrap_clamp_and_reset() {
        let near = |actual: f32, expected: f32| {
            assert!((actual - expected).abs() < 0.001, "{actual} != {expected}")
        };
        let mut pose = IntentionalPose::default();
        pose.apply(SemanticInput::GazeDelta([1.0, -1.0]));
        near(pose.gaze_yaw_degrees(), -180.0);
        near(pose.gaze_pitch_degrees(), 89.0);
        pose.apply(SemanticInput::ViewDelta([0.5, 0.5]));
        near(pose.view_yaw_degrees(), 90.0);
        near(pose.view_pitch_degrees(), -45.0);
        pose.apply(SemanticInput::ViewDelta([2.0, -10.0]));
        near(pose.view_yaw_degrees(), 90.0);
        near(pose.view_pitch_degrees(), 89.0);
        pose.apply(SemanticInput::ResetPose);
        assert_eq!(pose, IntentionalPose::default());
    }
}
