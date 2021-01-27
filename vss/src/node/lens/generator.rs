use super::*;
use cgmath::Vector3;
use gfx::{self};
use gfx::{texture};

gfx_defines! {
    pipeline pipe {
        rt_color: gfx::RenderTarget<(gfx::format::R32_G32_B32_A32, gfx::format::Float)> = "rt_color",
        u_right: gfx::Global<[f32; 3]> = "u_right",
        u_up: gfx::Global<[f32; 3]> = "u_up",
        u_forward: gfx::Global<[f32; 3]> = "u_forward",
    }
}

pub struct NormalMapGenerator {
    pub cube_texture: gfx::handle::Texture<Resources, gfx::format::R32_G32_B32_A32>,
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

        let (cube_texture, _) = load_highp_cubemap_from_bytes(&mut factory, &[&[255; 16]; 6], 1).unwrap();
        let target = factory.view_texture_as_render_target(&cube_texture, 0, None).unwrap();

        NormalMapGenerator {
            cube_texture,
            pso,
            pso_data: pipe::Data {
                rt_color: target,
                u_right: [1.0, 0.0, 1.0],
                u_up: [0.0, 0.0, 0.0],
                u_forward: [0.0, 0.0, 0.0],
            },
        }
    }

    fn generate_side(&mut self, factory: &mut gfx_device_gl::Factory, encoder: &mut gfx::Encoder<gfx_device_gl::Resources, gfx_device_gl::CommandBuffer>, orientation: &[Vector3<f32>; 3], layer: u16){
        let target = factory.view_texture_as_render_target(&self.cube_texture, 0, Some(layer)).unwrap();
        self.pso_data = pipe::Data {
            rt_color: target,
            u_right: orientation[0].into(),
            u_up: orientation[1].into(),
            u_forward: orientation[2].into(),
        };
        encoder.draw(&gfx::Slice::from_vertex_count(6), &self.pso, &self.pso_data);
    }

    pub fn generate(&mut self, window: &Window, width: texture::Size){
        let mut factory = window.factory().borrow_mut();
        //let mut temp = Vec::with_capacity((16*width*width) as usize);
        //temp.resize((16*width*width) as usize, 0);
        let (cube_texture, _) = load_highp_cubemap_from_bytes(&mut factory, &[&[255; 16*1000*1000]; 6], 1000).unwrap();
        self.cube_texture = cube_texture;
        
        self.generate_side(&mut*factory, &mut*window.encoder().borrow_mut(), &[-Vector3::unit_z(), -Vector3::unit_y(),  Vector3::unit_x()], 0);
        self.generate_side(&mut*factory, &mut*window.encoder().borrow_mut(), &[ Vector3::unit_z(), -Vector3::unit_y(), -Vector3::unit_x()], 1);
        self.generate_side(&mut*factory, &mut*window.encoder().borrow_mut(), &[ Vector3::unit_x(),  Vector3::unit_z(),  Vector3::unit_y()], 2);
        self.generate_side(&mut*factory, &mut*window.encoder().borrow_mut(), &[ Vector3::unit_x(), -Vector3::unit_z(), -Vector3::unit_y()], 3);
        self.generate_side(&mut*factory, &mut*window.encoder().borrow_mut(), &[ Vector3::unit_x(), -Vector3::unit_y(),  Vector3::unit_z()], 4);
        self.generate_side(&mut*factory, &mut*window.encoder().borrow_mut(), &[-Vector3::unit_x(), -Vector3::unit_y(), -Vector3::unit_z()], 5);
    }
}
