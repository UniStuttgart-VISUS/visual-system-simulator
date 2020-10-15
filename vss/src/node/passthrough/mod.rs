use super::*;

/// A node that does dot alter anything.
pub struct Passthrough;

impl Node for Passthrough {
    fn new(_window: &Window) -> Self {
        Passthrough {}
    }

    fn update_io(
        &mut self,
        _window: &Window,
        source: (Option<NodeSource>, Option<NodeTarget>),
        _target_candidate: (Option<NodeSource>, Option<NodeTarget>),
    ) -> (Option<NodeSource>, Option<NodeTarget>) {
        source
    }

    fn render(&mut self, _window: &Window) {}
}
