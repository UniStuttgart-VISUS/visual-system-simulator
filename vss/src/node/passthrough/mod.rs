use crate::*;

/// A node that does dot alter anything.
pub struct Passthrough;

impl Node for Passthrough {
    fn new(_window: &Window) -> Self {
        Passthrough {}
    }

    fn update_io(
        &mut self,
        _window: &Window,
        source: (Option<DeviceSource>, Option<DeviceTarget>),
        _target_candidate: (Option<DeviceSource>, Option<DeviceTarget>),
    ) -> (Option<DeviceSource>, Option<DeviceTarget>) {
        source
    }

    fn render(&mut self, _window: &Window) {}
}
