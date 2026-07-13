use crate::{
    cataract::Cataract, eye_control::EyeControl, lens::Lens, peacock::PeacockCB, retina::Retina,
    variance::VarianceMeasure, vis_overlay::VisOverlay, AssetId, Flow, Node, NodeChanges,
};
use cgmath::Matrix4;
use std::{
    any::TypeId,
    collections::BTreeMap,
    marker::PhantomData,
    sync::{Arc, OnceLock},
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ParameterKind {
    Bool,
    F64,
    F32,
    I32,
    U32,
    String,
    Asset,
    Matrix,
    Point,
}

#[derive(Clone, Debug)]
pub enum ParameterValue {
    Bool(bool),
    F64(f64),
    F32(f32),
    I32(i32),
    U32(u32),
    String(String),
    Asset(AssetId),
    Matrix(Matrix4<f32>),
    Point([f64; 2]),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ApplyResult {
    Unchanged,
    Changed,
    Missing,
}

pub trait TypedParameterValue: Clone + PartialEq + Send + Sync + 'static {
    const KIND: ParameterKind;
    fn into_value(self) -> ParameterValue;
    fn from_value(value: &ParameterValue) -> Option<Self>;
}

macro_rules! parameter_value {
    ($ty:ty, $kind:ident) => {
        impl TypedParameterValue for $ty {
            const KIND: ParameterKind = ParameterKind::$kind;
            fn into_value(self) -> ParameterValue {
                ParameterValue::$kind(self)
            }
            fn from_value(value: &ParameterValue) -> Option<Self> {
                match value {
                    ParameterValue::$kind(value) => Some(value.clone()),
                    _ => None,
                }
            }
        }
    };
}
parameter_value!(bool, Bool);
parameter_value!(f64, F64);
parameter_value!(f32, F32);
parameter_value!(i32, I32);
parameter_value!(u32, U32);
parameter_value!(String, String);
parameter_value!(AssetId, Asset);
parameter_value!(Matrix4<f32>, Matrix);
parameter_value!([f64; 2], Point);

pub struct ParameterId<N, T> {
    id: &'static str,
    field: Option<fn(&mut N) -> &mut T>,
    custom_set: Option<fn(&mut N, T) -> bool>,
    marker: PhantomData<fn() -> (N, T)>,
}

impl<N, T> Copy for ParameterId<N, T> {}
impl<N, T> Clone for ParameterId<N, T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<N, T: PartialEq> ParameterId<N, T> {
    pub const fn for_node(id: &'static str, field: fn(&mut N) -> &mut T) -> Self {
        Self {
            id,
            field: Some(field),
            custom_set: None,
            marker: PhantomData,
        }
    }
}

impl<N, T> ParameterId<N, T> {
    pub const fn with_setter(id: &'static str, set: fn(&mut N, T) -> bool) -> Self {
        Self {
            id,
            field: None,
            custom_set: Some(set),
            marker: PhantomData,
        }
    }
    pub const fn as_str(self) -> &'static str {
        self.id
    }
    fn apply(self, node: &mut N, value: T) -> bool
    where
        T: PartialEq,
    {
        if let Some(set) = self.custom_set {
            return set(node, value);
        }
        let current = self.field.expect("parameter has a setter")(node);
        if *current == value {
            false
        } else {
            *current = value;
            true
        }
    }
}

impl<N: Node + 'static, T: TypedParameterValue> ParameterId<N, T> {
    pub fn descriptor(self) -> ParameterDescriptor {
        ParameterDescriptor {
            id: self.id,
            kind: T::KIND,
            node_type: Some(TypeId::of::<N>()),
            present: Arc::new(|flow| flow.unique_node_index::<N>().is_some()),
            apply: Arc::new(move |flow, value| {
                let value = T::from_value(value).expect("parameter value kind was validated");
                match flow.try_with_unique_node_mut::<N, _>(|node| self.apply(node, value)) {
                    Some(true) => ApplyResult::Changed,
                    Some(false) => ApplyResult::Unchanged,
                    None => ApplyResult::Missing,
                }
            }),
        }
    }
    pub fn handle(self) -> ParameterHandle<N, T> {
        ParameterHandle(self)
    }
}

pub struct ParameterHandle<N, T>(ParameterId<N, T>);
impl<N: Node + 'static, T: TypedParameterValue> ParameterHandle<N, T> {
    pub fn resolve(self, flow: &Flow) -> Option<ResolvedParameterHandle<'_, N, T>> {
        Some(ResolvedParameterHandle {
            flow,
            index: flow.unique_node_index::<N>()?,
            id: self.0,
        })
    }
}

pub struct ResolvedParameterHandle<'a, N, T> {
    flow: &'a Flow,
    index: usize,
    id: ParameterId<N, T>,
}
impl<N: Node + 'static, T: TypedParameterValue> ResolvedParameterHandle<'_, N, T> {
    pub fn set(&self, value: T) -> NodeChanges {
        let changed = self
            .flow
            .with_node_at_mut(self.index, |node: &mut N| self.id.apply(node, value));
        if changed == Some(true) {
            self.flow.configure_node_at(self.index)
        } else {
            NodeChanges::empty()
        }
    }
}

pub struct FlowParameterId<T> {
    id: &'static str,
    set: fn(&Flow, T) -> bool,
}
impl<T> Copy for FlowParameterId<T> {}
impl<T> Clone for FlowParameterId<T> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<T> FlowParameterId<T> {
    pub const fn new(id: &'static str, set: fn(&Flow, T) -> bool) -> Self {
        Self { id, set }
    }
    pub const fn as_str(self) -> &'static str {
        self.id
    }
}
impl<T: TypedParameterValue> FlowParameterId<T> {
    pub fn descriptor(self) -> ParameterDescriptor {
        ParameterDescriptor {
            id: self.id,
            kind: T::KIND,
            node_type: None,
            present: Arc::new(|_| true),
            apply: Arc::new(move |flow, value| {
                if (self.set)(
                    flow,
                    T::from_value(value).expect("parameter value kind was validated"),
                ) {
                    ApplyResult::Changed
                } else {
                    ApplyResult::Unchanged
                }
            }),
        }
    }
    pub fn handle(self) -> FlowParameterHandle<T> {
        FlowParameterHandle(self)
    }
}

pub struct FlowParameterHandle<T>(FlowParameterId<T>);
impl<T: TypedParameterValue> FlowParameterHandle<T> {
    pub fn set(&self, flow: &Flow, value: T) -> NodeChanges {
        (self.0.set)(flow, value);
        NodeChanges::empty()
    }
}

pub trait PatchParameter<T: TypedParameterValue>: Copy {
    fn id(self) -> &'static str;
    fn descriptor(self) -> ParameterDescriptor;
}
impl<N: Node + 'static, T: TypedParameterValue> PatchParameter<T> for ParameterId<N, T> {
    fn id(self) -> &'static str {
        self.id
    }
    fn descriptor(self) -> ParameterDescriptor {
        self.descriptor()
    }
}
impl<T: TypedParameterValue> PatchParameter<T> for FlowParameterId<T> {
    fn id(self) -> &'static str {
        self.id
    }
    fn descriptor(self) -> ParameterDescriptor {
        self.descriptor()
    }
}

#[derive(Clone)]
pub struct ParameterDescriptor {
    id: &'static str,
    kind: ParameterKind,
    node_type: Option<TypeId>,
    present: Arc<dyn Fn(&Flow) -> bool + Send + Sync>,
    apply: Arc<dyn Fn(&Flow, &ParameterValue) -> ApplyResult + Send + Sync>,
}
impl ParameterDescriptor {
    pub const fn id(&self) -> &'static str {
        self.id
    }
    pub const fn kind(&self) -> ParameterKind {
        self.kind
    }
}

pub trait Parameters {
    fn parameters() -> &'static [ParameterDescriptor];
}

#[derive(Default)]
pub struct ParameterPatch {
    values: BTreeMap<&'static str, (ParameterDescriptor, ParameterValue)>,
}
impl ParameterPatch {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn set<T: TypedParameterValue, I: PatchParameter<T>>(
        &mut self,
        id: I,
        value: T,
    ) -> Result<(), &'static str> {
        if self
            .values
            .insert(id.id(), (id.descriptor(), value.into_value()))
            .is_some()
        {
            Err("parameter written more than once")
        } else {
            Ok(())
        }
    }
    pub fn set_erased(
        &mut self,
        descriptor: ParameterDescriptor,
        value: ParameterValue,
    ) -> Result<(), &'static str> {
        let id = descriptor.id;
        if self.values.insert(id, (descriptor, value)).is_some() {
            Err("parameter written more than once")
        } else {
            Ok(())
        }
    }
    pub fn apply(&self, flow: &Flow) -> NodeChanges {
        self.apply_with_warnings(flow).0
    }
    pub fn missing<'a>(&'a self, flow: &Flow) -> Vec<&'static str> {
        self.values
            .values()
            .filter_map(|(descriptor, _)| (!(descriptor.present)(flow)).then_some(descriptor.id))
            .collect()
    }
    pub fn apply_with_warnings(&self, flow: &Flow) -> (NodeChanges, Vec<&'static str>) {
        let missing = self.missing(flow);
        if !missing.is_empty() {
            return (NodeChanges::empty(), missing);
        }
        let mut changed = Vec::new();
        for (descriptor, value) in self.values.values() {
            match (descriptor.apply)(flow, value) {
                ApplyResult::Changed => {
                    if let Some(node_type) = descriptor.node_type {
                        if !changed.contains(&node_type) {
                            changed.push(node_type);
                        }
                    }
                }
                ApplyResult::Missing => unreachable!("patch targets were preflighted"),
                ApplyResult::Unchanged => {}
            }
        }
        (flow.configure_node_types(&changed), Vec::new())
    }
}

pub fn registry() -> &'static [ParameterDescriptor] {
    static REGISTRY: OnceLock<Vec<ParameterDescriptor>> = OnceLock::new();
    REGISTRY.get_or_init(|| {
        let mut result = Vec::new();
        result.extend_from_slice(Flow::parameters());
        result.extend_from_slice(Cataract::parameters());
        result.extend_from_slice(PeacockCB::parameters());
        result.extend_from_slice(EyeControl::parameters());
        result.extend_from_slice(Lens::parameters());
        result.extend_from_slice(Retina::parameters());
        result.extend_from_slice(VarianceMeasure::parameters());
        result.extend_from_slice(VisOverlay::parameters());
        result.sort_by_key(ParameterDescriptor::id);
        assert!(
            result.windows(2).all(|pair| pair[0].id != pair[1].id),
            "duplicate parameter id"
        );
        result
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{NodeSlots, RenderContext, RenderTexture, Texture};

    struct TestNode {
        first: i32,
        second: i32,
        configured: usize,
    }
    impl Node for TestNode {
        fn name(&self) -> &'static str {
            "TestNode"
        }
        fn negociate_slots(
            &mut self,
            _: &RenderContext,
            _: NodeSlots,
            _: &mut Option<Texture>,
        ) -> NodeSlots {
            panic!("not used")
        }
        fn configure(&mut self) -> NodeChanges {
            self.configured += 1;
            NodeChanges::OUTPUT
        }
        fn render(
            &mut self,
            _: &RenderContext,
            _: &mut wgpu::CommandEncoder,
            _: Option<&RenderTexture>,
        ) {
        }
    }
    const FIRST: ParameterId<TestNode, i32> =
        ParameterId::for_node("test.first", |node| &mut node.first);
    const SECOND: ParameterId<TestNode, i32> =
        ParameterId::for_node("test.second", |node| &mut node.second);
    struct MissingNode {
        value: i32,
    }
    impl Node for MissingNode {
        fn name(&self) -> &'static str {
            "MissingNode"
        }
        fn negociate_slots(
            &mut self,
            _: &RenderContext,
            _: NodeSlots,
            _: &mut Option<Texture>,
        ) -> NodeSlots {
            panic!("not used")
        }
        fn render(
            &mut self,
            _: &RenderContext,
            _: &mut wgpu::CommandEncoder,
            _: Option<&RenderTexture>,
        ) {
        }
    }
    const MISSING: ParameterId<MissingNode, i32> =
        ParameterId::for_node("test.missing", |node| &mut node.value);

    fn flow() -> Flow {
        let mut flow = Flow::new();
        flow.add_node(Box::new(TestNode {
            first: 0,
            second: 0,
            configured: 0,
        }));
        flow
    }

    #[test]
    fn handle_is_direct_and_skips_unchanged_values() {
        let flow = flow();
        let handle = FIRST.handle().resolve(&flow).unwrap();
        assert_eq!(handle.set(4), NodeChanges::OUTPUT);
        assert_eq!(handle.set(4), NodeChanges::empty());
        flow.with_node_at_mut(
            flow.unique_node_index::<TestNode>().unwrap(),
            |node: &mut TestNode| assert_eq!((node.first, node.configured), (4, 1)),
        )
        .unwrap();
    }

    #[test]
    fn patch_configures_a_changed_node_once() {
        let flow = flow();
        let mut patch = ParameterPatch::new();
        patch.set(FIRST, 1).unwrap();
        patch.set(SECOND, 2).unwrap();
        assert_eq!(patch.apply(&flow), NodeChanges::OUTPUT);
        flow.with_node_at_mut(
            flow.unique_node_index::<TestNode>().unwrap(),
            |node: &mut TestNode| assert_eq!((node.first, node.second, node.configured), (1, 2, 1)),
        )
        .unwrap();
    }

    #[test]
    fn missing_node_is_reported_without_panicking() {
        let flow = flow();
        let mut patch = ParameterPatch::new();
        patch.set(FIRST, 7).unwrap();
        patch.set(MISSING, 1).unwrap();
        let (changes, missing) = patch.apply_with_warnings(&flow);
        assert_eq!(changes, NodeChanges::empty());
        assert_eq!(missing, ["test.missing"]);
        flow.with_node_at_mut(
            flow.unique_node_index::<TestNode>().unwrap(),
            |node: &mut TestNode| assert_eq!(node.first, 0),
        )
        .unwrap();
    }

    #[test]
    fn flow_handle_and_patch_use_the_direct_path() {
        let flow = Flow::new();
        assert_eq!(
            crate::flow::GAZE.handle().set(&flow, [1.0, 2.0]),
            NodeChanges::empty()
        );
        assert_eq!(flow.gaze(), [1.0, 2.0]);
        let mut patch = ParameterPatch::new();
        patch.set(crate::flow::VIEW, [3.0, 4.0]).unwrap();
        assert_eq!(patch.apply(&flow), NodeChanges::empty());
        assert_eq!(flow.view(), [3.0, 4.0]);
    }

    #[test]
    fn registry_ids_are_unique() {
        assert!(registry()
            .windows(2)
            .all(|pair| pair[0].id() != pair[1].id()));
    }

    #[test]
    fn registry_ids_are_dotted_kebab_case() {
        fn is_kebab_case(segment: &str) -> bool {
            !segment.is_empty()
                && segment.split('-').all(|word| {
                    !word.is_empty()
                        && word
                            .bytes()
                            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit())
                })
        }

        for parameter in registry() {
            let id = parameter.id();
            assert!(
                id.split('.').all(is_kebab_case),
                "parameter id `{id}` is not dotted kebab-case"
            );
        }
    }
}
