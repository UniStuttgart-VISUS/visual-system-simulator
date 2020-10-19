use super::*;

/// A node that does dot alter anything.
pub struct Passthrough;

impl Node for Passthrough {
    fn new(_window: &Window) -> Self {
        Passthrough {}
    }

    fn negociate_slots(&mut self, _window: &Window, slots: NodeSlots) -> NodeSlots {
        slots.to_passthrough()
    }

    fn render(&mut self, _window: &Window) {}
}
