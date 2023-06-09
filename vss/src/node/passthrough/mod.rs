use super::*;

/// A node that does dot alter anything.
pub struct Passthrough;

impl Passthrough {
    pub fn new(_surface: &Surface) -> Self {
        Passthrough {}
    }
}

impl Node for Passthrough {
    fn negociate_slots(
        &mut self,
        _surface: &Surface,
        slots: NodeSlots,
        _original_image: &mut Option<Texture>,
    ) -> NodeSlots {
        slots.to_passthrough()
    }

    fn render(
        &mut self,
        _surface: &Surface,
        _encoder: &mut CommandEncoder,
        _screen: Option<&RenderTexture>,
    ) {
    }
}
