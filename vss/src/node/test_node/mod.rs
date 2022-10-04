use super::*;
use wgpu::{util::DeviceExt, CommandEncoder};

// gfx_defines! {
//     pipeline pipe {
//         s_source: gfx::TextureSampler<[f32; 4]> = "s_color",
//         rt_color: gfx::RenderTarget<ColorFormat> = "rt_color",
//     }
// }

struct TestNodeUniforms{
    test_color: [f32; 4],
}

pub struct TestNode {
    pipeline: wgpu::RenderPipeline,
    test_texture: texture::Texture,
    texture_bind_group: wgpu::BindGroup,
    uniforms: TestNodeUniforms,
    uniforms_buffer: wgpu::Buffer,
    uniforms_bind_group: wgpu::BindGroup,
}

impl Node for TestNode {
    fn new(window: &window::Window) -> Self {
        let device = window.device().borrow_mut();
        let queue = window.queue().borrow_mut();

        let uniforms = TestNodeUniforms{test_color: [1.0, 1.0, 0.4, 1.0]};

        let uniforms_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Uniform Buffer"),
            contents: unsafe { any_as_u8_slice(&uniforms) },
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let uniforms_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
                label: Some("uniforms_bind_group_layout"),
            });

        let uniforms_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &uniforms_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniforms_buffer.as_entire_binding(),
            }],
            label: Some("uniforms_bind_group"),
        });

        let raw_data = vec![64; (100*100*4) as usize];
        let test_texture = load_texture_from_bytes(
            &device,
            &queue,
            raw_data.as_slice(),
            100,
            100,
            Some("TestNode Texture")).unwrap();
        
        let test_sampler = create_sampler_linear(&device).unwrap();

        let (texture_bind_group_layout, texture_bind_group) = test_texture.create_bind_group(&device, &test_sampler);

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("TestNode Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("mod.wgsl").into()),
        });

        let pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("TestNode Render Pipeline Layout"),
                bind_group_layouts: &[&texture_bind_group_layout, &uniforms_bind_group_layout],
                push_constant_ranges: &[],
        });
        
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("TestNode Render Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: window.surface_config().borrow().format,
                    blend: Some(wgpu::BlendState {
                        color: wgpu::BlendComponent::REPLACE,
                        alpha: wgpu::BlendComponent::REPLACE,
                    }),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                // Setting this to anything other than Fill requires Features::POLYGON_MODE_LINE
                // or Features::POLYGON_MODE_POINT
                polygon_mode: wgpu::PolygonMode::Fill,
                // Requires Features::DEPTH_CLIP_CONTROL
                unclipped_depth: false,
                // Requires Features::CONSERVATIVE_RASTERIZATION
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            // If the pipeline will be used with a multiview render pass, this
            // indicates how many array layers the attachments will have.
            multiview: None,
        });

        TestNode {
            pipeline,
            test_texture,
            texture_bind_group,
            uniforms,
            uniforms_buffer,
            uniforms_bind_group,
        }
    }

    // fn negociate_slots(&mut self, window: &window::Window, slots: NodeSlots) -> NodeSlots {
    //     let slots = slots.to_color_input(window).to_color_output(window);
    //     self.pso_data.s_source = slots.as_color_view();
    //     self.pso_data.rt_color = slots.as_color();

    //     slots
    // }

    fn render(&mut self, window: &window::Window, encoder: &mut CommandEncoder, screen: &RenderTexture) {
        let mut render_pass = create_render_pass(encoder, screen);
        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, &self.texture_bind_group, &[]);
        render_pass.set_bind_group(1, &self.uniforms_bind_group, &[]);
        render_pass.draw(0..6, 0..1);
    }
}
