use std::borrow::BorrowMut;

use super::*;
use cgmath::Matrix4;
use cgmath::Rad;
use std::sync::OnceLock;
use wgpu::CommandEncoder;

#[repr(C)]
struct Uniforms {
    hive_rotation: [[f32; 4]; 4],

    resolution_in: [f32; 2],
    hive_position: [f32; 2],

    hive_visible: i32,
    flow_idx: i32,

    heat_scale: f32,

    base_image: i32,
    combination_function: i32,
    mix_type: i32,
    colormap_type: i32,

    _padding: i32,
}

#[derive(Copy, Clone, Debug, PartialEq, Default)]
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

#[derive(Copy, Clone, Debug, PartialEq, Default)]
enum ColorMapType {
    #[default]
    Viridis,
    Turbo,
    Grayscale,
}

#[derive(Copy, Clone, Debug, PartialEq, Default)]
enum BaseImage {
    #[default]
    Output,
    Original,
    Ganglion,
}

#[derive(Copy, Clone, Debug, PartialEq, Default)]
struct VisualizationType {
    pub base_image: BaseImage,
    pub combination_function: CombinationFunction,
    pub mix_type: MixType,
    pub color_map_type: ColorMapType,
}

pub struct MetricOverlay {
    config: MetricOverlayConfig,
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
    bees_visible: bool,
}

pub struct MetricOverlayConfig {
    eye_idx: i32,
    vis_type: VisualizationType,
    heat_scale: f32,
}

impl Default for MetricOverlayConfig {
    fn default() -> Self {
        Self {
            eye_idx: 0,
            vis_type: VisualizationType::default(),
            heat_scale: 1.0,
        }
    }
}

pub const EYE: ParameterId<MetricOverlay, i32> =
    ParameterId::for_node("metric-overlay.eye", |n| &mut n.config.eye_idx);
pub const HEAT_SCALE: ParameterId<MetricOverlay, f32> =
    ParameterId::for_node("metric-overlay.heat-scale", |n| &mut n.config.heat_scale);
pub const BASE_IMAGE: ParameterId<MetricOverlay, i32> =
    ParameterId::with_setter("metric-overlay.base-image", |n, v| {
        set_enum(
            &mut n.config.vis_type.base_image,
            v,
            &[BaseImage::Output, BaseImage::Original, BaseImage::Ganglion],
        )
    });
pub const MIX_TYPE: ParameterId<MetricOverlay, i32> =
    ParameterId::with_setter("metric-overlay.mix-type", |n, v| {
        set_enum(
            &mut n.config.vis_type.mix_type,
            v,
            &[
                MixType::BaseImageOnly,
                MixType::ColorMapOnly,
                MixType::OverlayThreshold,
            ],
        )
    });
pub const COLOR_MAP: ParameterId<MetricOverlay, i32> =
    ParameterId::with_setter("metric-overlay.color-map", |n, v| {
        set_enum(
            &mut n.config.vis_type.color_map_type,
            v,
            &[
                ColorMapType::Viridis,
                ColorMapType::Turbo,
                ColorMapType::Grayscale,
            ],
        )
    });
pub const COLOR_FUNCTION: ParameterId<MetricOverlay, i32> =
    ParameterId::with_setter("metric-overlay.color-function", |n, v| {
        set_enum(
            &mut n.config.vis_type.combination_function,
            v,
            &[
                CombinationFunction::AbsoluteErrorRGBVectorLength,
                CombinationFunction::AbsoluteErrorXYVectorLength,
                CombinationFunction::AbsoluteErrorRGBXYVectorLength,
                CombinationFunction::UncertaintyRGBVectorLength,
                CombinationFunction::UncertaintyXYVectorLength,
                CombinationFunction::UncertaintyRGBXYVectorLength,
                CombinationFunction::UncertaintyGenVar,
            ],
        )
    });
fn set_enum<T: Copy + PartialEq>(target: &mut T, value: i32, values: &[T]) -> bool {
    let value = *values
        .get(value as usize)
        .expect("invalid visualization enum value");
    if *target == value {
        false
    } else {
        *target = value;
        true
    }
}
impl Parameters for MetricOverlay {
    fn parameters() -> &'static [ParameterDescriptor] {
        static P: OnceLock<Vec<ParameterDescriptor>> = OnceLock::new();
        P.get_or_init(|| {
            vec![
                EYE.descriptor(),
                BASE_IMAGE.descriptor(),
                MIX_TYPE.descriptor(),
                COLOR_MAP.descriptor(),
                COLOR_FUNCTION.descriptor(),
                HEAT_SCALE.descriptor(),
            ]
        })
    }
}

impl MetricOverlay {
    pub fn new(context: &RenderContext) -> Self {
        let device = context.device();
        let queue = context.queue();

        let uniforms = ShaderUniforms::new(
            device,
            Uniforms {
                resolution_in: [1.0, 1.0],
                flow_idx: 0,

                heat_scale: 1.0,

                hive_rotation: [[0.0; 4]; 4],
                hive_position: [0.0; 2],
                hive_visible: 0,

                base_image: 0,
                combination_function: 0,
                mix_type: 0,
                colormap_type: 0,

                _padding: 0,
            },
        );

        let (sources_bind_group_layout, sources_bind_group) =
            create_color_sources_bind_group(device, queue, "MetricOverlayNode");

        let original_tex =
            placeholder_texture(device, queue, Some("MetricOverlayNode s_original")).unwrap();
        let (original_bind_group_layout, original_bind_group) =
            original_tex.create_bind_group(device);

        let render_target = RenderTexture::empty_color_with_format(
            device,
            context.output_format(),
            Some("MetricOverlayNode render_target"),
        );

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("MetricOverlayNode Shader"),
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
            &[blended_color_state(context.output_format())],
            None,
            Some("MetricOverlayNode Render Pipeline"),
        );

        MetricOverlay {
            config: MetricOverlayConfig::default(),
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
            bees_visible: false,
            //  previous_mouse_position: (0.0, 0.0),
        }
    }
}

impl Node for MetricOverlay {
    fn name(&self) -> &'static str {
        "MetricOverlay"
    }

    fn negociate_slots(
        &mut self,
        context: &RenderContext,
        slots: NodeSlots,
        original_image: &mut Option<Texture>,
    ) -> NodeSlots {
        let slots = slots
            .to_color_metrics_input(context)
            .to_color_output(context, "MetricOverlayNode");
        let device = context.device();
        let queue = context.queue();

        self.uniforms.data.resolution_in = slots.input_size_f32();

        self.sources_bind_group = slots.as_all_colors_source(device, queue);
        self.render_target = slots.as_color_target();
        if let Some(tex) = original_image.borrow_mut() {
            (_, self.original_bind_group) = tex.create_bind_group(device);
        }

        slots
    }

    fn configure(&mut self) -> NodeChanges {
        let eye_idx = self.config.eye_idx as u32;
        let output_changed = self.eye_idx != eye_idx
            || self.vis_type != self.config.vis_type
            || self.heat_scale != self.config.heat_scale;

        self.eye_idx = eye_idx;
        self.vis_type = self.config.vis_type;
        self.heat_scale = self.config.heat_scale;

        NodeChanges::from_output_slots(output_changed, false)
    }

    fn render(
        &mut self,
        context: &RenderContext,
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
            self.hive_rot * Matrix4::from_angle_x(Rad(speed * context.delta_t() / 1_000_000.0));
        self.hive_rot = self.hive_rot
            * Matrix4::from_angle_y(Rad(0.7 * speed * context.delta_t() / 1_000_000.0));
        self.hive_rot = self.hive_rot
            * Matrix4::from_angle_z(Rad(0.2 * speed * context.delta_t() / 1_000_000.0));

        self.uniforms.data.hive_rotation = self.hive_rot.into();

        self.uniforms.upload(context.queue());

        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("MetricOverlayNode render_pass"),
            color_attachments: &[screen
                .unwrap_or(&self.render_target)
                .to_color_attachment(Some(CLEAR_COLOR))],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });

        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, &self.uniforms.bind_group, &[]);
        render_pass.set_bind_group(1, &self.sources_bind_group, &[]);
        render_pass.set_bind_group(2, &self.original_bind_group, &[]);
        render_pass.draw(0..6, 0..1);
    }
}
