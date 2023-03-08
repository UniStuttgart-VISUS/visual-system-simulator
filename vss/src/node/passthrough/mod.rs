use super::*;

/// A node that does dot alter anything.
pub struct Passthrough;

impl Node for Passthrough {
    fn new(_surface: &Surface) -> Self {
        Passthrough {}
    }

    fn negociate_slots(&mut self, _surface: &Surface, slots: NodeSlots) -> NodeSlots {
        slots.to_passthrough()
    }

    fn render(&mut self, surface: &Surface, encoder: &mut CommandEncoder, screen: Option<&RenderTexture>) {}
}
