use super::*;

/// A node that does dot alter anything.
pub struct Passthrough;

impl Passthrough {
    pub fn new(_context: &RenderContext) -> Self {
        Passthrough {}
    }
}

impl Node for Passthrough {
    fn name(&self) -> &'static str {
        "Passthrough"
    }

    fn negociate_slots(
        &mut self,
        _context: &RenderContext,
        slots: NodeSlots,
        _original_image: &mut Option<Texture>,
    ) -> NodeSlots {
        slots.to_passthrough()
    }

    fn render(
        &mut self,
        _context: &RenderContext,
        _encoder: &mut CommandEncoder,
        _screen: Option<&RenderTexture>,
    ) {
    }
}
