use super::*;
use gfx;
use gfx::{texture};

gfx_defines! {
    pipeline pipe {
        rt_color: gfx::RenderTarget<(gfx::format::R32_G32_B32_A32, gfx::format::Float)> = "rt_color",
    }
}

pub struct NormalMapGenerator {
    pub texture: gfx::handle::Texture<Resources, gfx::format::R32_G32_B32_A32>,
    pso: gfx::PipelineState<Resources, pipe::Meta>,
    pso_data: pipe::Data<Resources>,
}

impl NormalMapGenerator {
    pub fn new(window: &Window) -> Self {
        let mut factory = window.factory().borrow_mut();
        let pso = factory
            .create_pipeline_simple(
                &include_glsl!("../mod.vert"),
                &include_glsl!("generator.frag"),
                pipe::new(),
            )
            .unwrap();

        let (texture, _, dst) = factory.create_render_target(1, 1).unwrap();

        NormalMapGenerator {
            texture,
            pso,
            pso_data: pipe::Data {
                rt_color: dst,
            },
        }
    }

    pub fn generate(&mut self, window: &Window, width: texture::Size, height: texture::Size){
        let mut factory = window.factory().borrow_mut();
        let (texture, _, dst) = factory.create_render_target(width, height).unwrap();
        self.texture = texture;
        self.pso_data = pipe::Data {rt_color: dst};
        
        let mut encoder = window.encoder().borrow_mut();
        encoder.draw(&gfx::Slice::from_vertex_count(6), &self.pso, &self.pso_data);
    }
}
