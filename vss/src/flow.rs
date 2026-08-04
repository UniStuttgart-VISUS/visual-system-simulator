use crate::*;
use cgmath::Matrix4;
use cgmath::Vector3;
use std::{
    any::{Any, TypeId},
    cell::{RefCell, RefMut},
    sync::OnceLock,
};
use wgpu::CommandEncoder;

pub const GAZE: FlowParameterId<[f64; 2]> =
    FlowParameterId::new("gaze", |flow, value| flow.set_flow_parameter("gaze", value));
pub const VIEW: FlowParameterId<[f64; 2]> =
    FlowParameterId::new("view", |flow, value| flow.set_flow_parameter("view", value));

/// Represents properties of eye input (perspetive and tracking).
#[derive(Clone, Debug)]
pub struct EyeInput {
    pub position: Vector3<f32>,
    pub view: Matrix4<f32>,
    pub proj: Matrix4<f32>,
    pub gaze: Vector3<f32>,
}

/// Represents properties of mouse input.
#[derive(Clone, Debug, Default)]
pub struct MouseInput {
    pub position: (f32, f32),
    pub left_button: bool,
    pub right_button: bool,
}

/// A flow encapsulates simulation nodes, i.e., all simulation and rendering.
pub struct Flow {
    nodes: RefCell<Vec<Box<dyn Node>>>,
    eye: RefCell<EyeInput>,
    gaze: RefCell<[f64; 2]>,
    view: RefCell<[f64; 2]>,
}

impl Flow {
    pub fn new() -> Self {
        Flow {
            nodes: RefCell::new(Vec::new()),
            eye: RefCell::new(EyeInput {
                position: Vector3::new(0.0, 0.0, 0.0),
                view: Matrix4::from_scale(1.0),
                proj: cgmath::perspective(cgmath::Deg(70.0), 1.0, 0.05, 1000.0),
                gaze: Vector3::new(0.0, 0.0, 1.0),
            }),
            gaze: RefCell::new([0.0, 0.0]),
            view: RefCell::new([0.0, 0.0]),
        }
    }

    pub fn eye_mut(&self) -> RefMut<'_, EyeInput> {
        self.eye.borrow_mut()
    }

    pub(crate) fn set_flow_parameter(&self, id: &str, value: [f64; 2]) -> bool {
        match id {
            "gaze" if self.gaze() != value => *self.gaze.borrow_mut() = value,
            "view" if self.view() != value => *self.view.borrow_mut() = value,
            "gaze" | "view" => return false,
            _ => panic!("unknown flow parameter {id}"),
        }
        true
    }

    pub fn gaze(&self) -> [f64; 2] {
        *self.gaze.borrow()
    }

    pub fn view(&self) -> [f64; 2] {
        *self.view.borrow()
    }

    pub fn add_node(&mut self, node: Box<dyn Node>) {
        self.nodes.borrow_mut().push(node);
    }

    pub fn replace_node(&self, index: usize, node: Box<dyn Node>) {
        self.nodes.borrow_mut()[index] = node;
    }

    pub fn nodes_len(&self) -> usize {
        self.nodes.borrow().len()
    }

    pub fn negociate_slots(&self, context: &RenderContext) {
        let mut slot_a = NodeSlots::new();
        let mut slot_b = NodeSlots::new();
        let mut original_image: Option<Texture> = None;
        let nodes_len = self.nodes.borrow().len();
        for (idx, node) in self.nodes.borrow_mut().iter_mut().enumerate() {
            let suggested_slot = if idx + 1 == nodes_len {
                // Suggest window as final output.
                let device = context.device();

                let width = context.width();
                let height = context.height();

                let color_target = RenderTexture::create_color_with_format(
                    device,
                    width,
                    height,
                    context.output_format(),
                    Some("flow_negociate_slots color"),
                );

                let output_slot = Slot::Color {
                    color_source: color_target.as_texture(),
                    color_target,
                };

                NodeSlots::new_io(slot_b.take_output(), output_slot)
            } else {
                // Suggest reusing output of the pre-predecessor.
                NodeSlots::new_io(slot_b.take_output(), slot_a.take_output())
            };
            // Negociate and swap.
            slot_a = node.negociate_slots(context, suggested_slot, &mut original_image);
            std::mem::swap(&mut slot_a, &mut slot_b);
        }
    }

    pub(crate) fn try_with_unique_node_mut<N: Node + 'static, R>(
        &self,
        apply: impl FnOnce(&mut N) -> R,
    ) -> Option<R> {
        let mut nodes = self.nodes.borrow_mut();
        let mut matches = nodes
            .iter_mut()
            .filter_map(|node| (node.as_mut() as &mut dyn Any).downcast_mut::<N>());
        let target = matches.next();
        assert!(
            matches.next().is_none(),
            "duplicate parameter target node type"
        );
        target.map(apply)
    }

    pub(crate) fn unique_node_index<N: Node + 'static>(&self) -> Option<usize> {
        let nodes = self.nodes.borrow();
        let mut matches = nodes
            .iter()
            .enumerate()
            .filter(|(_, node)| node.as_ref().type_id() == TypeId::of::<N>());
        let index = matches.next()?.0;
        matches.next().is_none().then_some(index)
    }

    pub(crate) fn with_node_at_mut<N: Node + 'static, R>(
        &self,
        index: usize,
        apply: impl FnOnce(&mut N) -> R,
    ) -> Option<R> {
        let mut nodes = self.nodes.borrow_mut();
        Some(apply(
            (nodes.get_mut(index)?.as_mut() as &mut dyn Any).downcast_mut::<N>()?,
        ))
    }

    pub(crate) fn configure_node_at(&self, index: usize) -> NodeChanges {
        self.nodes
            .borrow_mut()
            .get_mut(index)
            .map_or(NodeChanges::empty(), |node| node.configure())
    }

    pub(crate) fn configure_node_types(&self, types: &[TypeId]) -> NodeChanges {
        let mut changes = NodeChanges::empty();
        let mut nodes = self.nodes.borrow_mut();
        for ty in types {
            let mut matches = nodes
                .iter_mut()
                .filter(|node| node.as_ref().type_id() == *ty);
            changes |= matches.next().expect("changed node is missing").configure();
            assert!(
                matches.next().is_none(),
                "duplicate parameter target node type"
            );
        }
        changes.normalized()
    }

    pub fn input(&self, mouse: &MouseInput) -> NodeChanges {
        // Propagate to nodes.
        let mut eye = self.eye.borrow().clone();
        let mut result = NodeChanges::empty();
        for node in self.nodes.borrow_mut().iter_mut().rev() {
            let (next_eye, changes) = node.input(&eye, mouse);
            eye = next_eye;
            result |= changes;
        }
        result.normalized()
    }

    pub fn render(
        &self,
        context: &RenderContext,
        encoder: &mut CommandEncoder,
        screen: &RenderTexture,
    ) {
        // Render all nodes.
        let nodes_len = self.nodes.borrow().len();
        if nodes_len == 0 {
            return;
        }
        let last_index = nodes_len - 1;
        for (idx, node) in self.nodes.borrow_mut().iter_mut().enumerate() {
            node.render(
                context,
                encoder,
                if idx == last_index {
                    Some(screen)
                } else {
                    None
                },
            );
        }
    }

    pub fn post_render(&self, context: &RenderContext) {
        for node in self.nodes.borrow_mut().iter_mut() {
            node.post_render(context);
        }
    }
}

impl Parameters for Flow {
    fn parameters() -> &'static [ParameterDescriptor] {
        static PARAMETERS: OnceLock<Vec<ParameterDescriptor>> = OnceLock::new();
        PARAMETERS.get_or_init(|| vec![GAZE.descriptor(), VIEW.descriptor()])
    }
}
