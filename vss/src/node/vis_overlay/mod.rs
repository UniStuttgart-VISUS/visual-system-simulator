use std::borrow::BorrowMut;

use super::*;
use cgmath::Matrix4;
use cgmath::Rad;
use wgpu::CommandEncoder;

struct Uniforms {
    hive_rotation: [[f32; 4]; 4],

    resolution_in: [f32; 2],
    hive_position: [f32; 2],

    hive_visible: i32,
    flow_idx: i32,

    heat_scale: f32,
    dir_calc_scale: f32,

    base_image: i32,
    combination_function: i32,
    mix_type: i32,
    colormap_type: i32,
}

#[derive(Copy, Clone, Debug, Default)]
enum CombinationFunction {
    #[default]
    AbsoluteErrorRGBVectorLength,
    AbsoluteErrorXYVectorLength,
    AbsoluteErrorRGBXYVectorLength,
    UncertaintyRGBVectorLength,
    UncertaintyXYVectorLength,
    UncertaintyRGBXYVectorLength,
    UncertaintyGenVar,
}

#[derive(Copy, Clone, Debug, PartialEq, Default)]
enum MixType {
    #[default]
    BaseImageOnly,
    ColorMapOnly,
    OverlayThreshold,
}

#[derive(Copy, Clone, Debug, Default)]
enum ColorMapType {
    #[default]
    Viridis,
    Turbo,
    Grayscale,
}

#[derive(Copy, Clone, Debug, Default)]
enum BaseImage {
    #[default]
    Output,
    Original,
    Ganglion,
    Variance,
}

#[derive(Copy, Clone, Debug, Default)]
struct VisualizationType {
    pub base_image: BaseImage,
    pub combination_function: CombinationFunction,
    pub mix_type: MixType,
    pub color_map_type: ColorMapType,
}

pub struct VisOverlay {
    hive_rot: Matrix4<f32>,
    pipeline: wgpu::RenderPipeline,
    uniforms: ShaderUniforms<Uniforms>,
    sources_bind_group: wgpu::BindGroup,
    original_bind_group: wgpu::BindGroup,
    render_target: RenderTexture,

    eye_idx: u32,
    vis_type: VisualizationType,
    heat_scale: f32,
    //previous_mouse_position: (f32, f32),
    highlight_position: (f32, f32),
    //bees_flying: bool,
    bees_visible: bool,
}

impl VisOverlay {
    pub fn new(surface: &Surface) -> Self {
        let device = surface.device();
        let queue = surface.queue();

        let uniforms = ShaderUniforms::new(
            device,
            Uniforms {
                resolution_in: [1.0, 1.0],
                flow_idx: 0,

                heat_scale: 1.0,
                dir_calc_scale: 1.0,

                hive_rotation: [[0.0; 4]; 4],
                hive_position: [0.0; 2],
                hive_visible: 0,

                base_image: 0,
                combination_function: 0,
                mix_type: 0,
                colormap_type: 0,
            },
        );

        let (sources_bind_group_layout, sources_bind_group) =
            create_color_sources_bind_group(device, queue, "VisOverlayNode");

        let original_tex =
            placeholder_texture(device, queue, Some("VisOverlayNode s_original")).unwrap();
        let (original_bind_group_layout, original_bind_group) =
            original_tex.create_bind_group(device);

        let render_target = placeholder_color_rt(device, Some("VisOverlayNode render_target"));

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("VisOverlayNode Shader"),
            source: wgpu::ShaderSource::Wgsl(
                concat!(
                    include_str!("../common.wgsl"),
                    include_str!("../vert.wgsl"),
                    include_str!("mod.wgsl")
                )
                .into(),
            ),
        });

        let pipeline = create_render_pipeline(
            device,
            &[&shader, &shader],
            &["vs_main", "fs_main"],
            &[
                &uniforms.bind_group_layout,
                &sources_bind_group_layout,
                &original_bind_group_layout,
            ],
            &[blended_color_state(COLOR_FORMAT)],
            None,
            Some("VisOverlayNode Render Pipeline"),
        );

        VisOverlay {
            hive_rot: Matrix4::from_angle_x(Rad(0.0)),
            pipeline,
            uniforms,
            sources_bind_group,
            original_bind_group,
            render_target,
            eye_idx: 0,
            vis_type: VisualizationType::default(),
            heat_scale: 1.0,
            highlight_position: (0.0, 0.0),
            // bees_flying: true,
            bees_visible: false,
            //  previous_mouse_position: (0.0, 0.0),
        }
    }
}

impl Node for VisOverlay {
    fn negociate_slots(
        &mut self,
        surface: &Surface,
        slots: NodeSlots,
        original_image: &mut Option<Texture>,
    ) -> NodeSlots {
        let slots = slots
            .to_color_input(surface)
            .to_color_output(surface, "VisOverlayNode");
        let device = surface.device();

        self.uniforms.data.resolution_in = slots.input_size_f32();

        self.sources_bind_group = slots.as_all_colors_source(device);
        self.render_target = slots.as_color_target();
        if let Some(tex) = original_image.borrow_mut() {
            (_, self.original_bind_group) = tex.create_bind_group(device);
        }

        slots
    }

    fn inspect(&mut self, inspector: &mut dyn Inspector) {
        inspector.begin_node("VisOverlay");
        inspector.mut_i32("flow_id", &mut self.uniforms.data.flow_idx);
        let mut file_base_image = self.vis_type.base_image as i32;
        if inspector.mut_i32("file_base_image", &mut file_base_image) {
            self.vis_type.base_image = match file_base_image {
                0 => BaseImage::Output,
                1 => BaseImage::Original,
                2 => BaseImage::Ganglion,
                _ => panic!("No BaseImage of {} found", file_base_image),
            };
        }

        let mut file_mix_type = self.vis_type.mix_type as i32;
        if inspector.mut_i32("file_mix_type", &mut file_mix_type) {
            self.vis_type.mix_type = match file_mix_type {
                0 => MixType::BaseImageOnly,
                1 => MixType::ColorMapOnly,
                2 => MixType::OverlayThreshold,
                _ => panic!("No MixType of {} found", file_mix_type),
            };
        }

        let mut file_cm_type = self.vis_type.color_map_type as i32;
        if inspector.mut_i32("file_cm_type", &mut file_cm_type) {
            self.vis_type.color_map_type = match file_cm_type {
                0 => ColorMapType::Viridis,
                1 => ColorMapType::Turbo,
                2 => ColorMapType::Grayscale,
                _ => panic!("No ColorMapType of {} found", file_cm_type),
            };
        }

        let mut file_cf = self.vis_type.combination_function as i32;
        if inspector.mut_i32("file_cf", &mut file_cf) {
            self.vis_type.combination_function = match file_cf {
                0 => CombinationFunction::AbsoluteErrorRGBVectorLength,
                1 => CombinationFunction::AbsoluteErrorXYVectorLength,
                2 => CombinationFunction::AbsoluteErrorRGBXYVectorLength,
                3 => CombinationFunction::UncertaintyRGBVectorLength,
                4 => CombinationFunction::UncertaintyXYVectorLength,
                5 => CombinationFunction::UncertaintyRGBXYVectorLength,
                6 => CombinationFunction::UncertaintyGenVar,
                _ => panic!("No CombinationFunction of {} found", file_cf),
            };
        }
        inspector.mut_f32("cm_scale", &mut self.heat_scale);

        inspector.end_node();
    }

    fn input(
        &mut self,
        perspective: &EyePerspective,
        vis_param: &VisualizationParameters,
    ) -> EyePerspective {
        self.uniforms.data.dir_calc_scale = vis_param.dir_calc_scale;
        perspective.clone()
    }

    fn render(
        &mut self,
        surface: &Surface,
        encoder: &mut CommandEncoder,
        screen: Option<&RenderTexture>,
    ) {
        let speed = 4.0;

        self.uniforms.data.heat_scale = self.heat_scale;
        self.uniforms.data.flow_idx = self.eye_idx as i32;
        self.uniforms.data.hive_position[0] = self.highlight_position.0;
        self.uniforms.data.hive_position[1] = self.highlight_position.1;
        self.uniforms.data.hive_visible = self.bees_visible as i32;

        self.uniforms.data.base_image = self.vis_type.base_image as i32;
        self.uniforms.data.combination_function = self.vis_type.combination_function as i32;
        self.uniforms.data.mix_type = self.vis_type.mix_type as i32;
        self.uniforms.data.colormap_type = self.vis_type.color_map_type as i32;

        self.hive_rot =
            self.hive_rot * Matrix4::from_angle_x(Rad(speed * surface.delta_t() / 1_000_000.0));
        self.hive_rot = self.hive_rot
            * Matrix4::from_angle_y(Rad(0.7 * speed * surface.delta_t() / 1_000_000.0));
        self.hive_rot = self.hive_rot
            * Matrix4::from_angle_z(Rad(0.2 * speed * surface.delta_t() / 1_000_000.0));

        self.uniforms.data.hive_rotation = self.hive_rot.into();

        self.uniforms.upload(surface.queue());

        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("VisOverlayNode render_pass"),
            color_attachments: &[screen
                .unwrap_or(&self.render_target)
                .to_color_attachment(Some(CLEAR_COLOR))],
            depth_stencil_attachment: None,
        });

        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, &self.uniforms.bind_group, &[]);
        render_pass.set_bind_group(1, &self.sources_bind_group, &[]);
        render_pass.set_bind_group(2, &self.original_bind_group, &[]);
        render_pass.draw(0..6, 0..1);
    }
}
