use crate::*;
use instant::Instant;
use std::{
    cell::{Cell, RefCell},
    sync::Arc,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EyeMode {
    Left,
    Both,
    Right,
}

impl EyeMode {
    pub fn flow_viewports(self) -> Vec<(usize, [f32; 4])> {
        match self {
            Self::Left => vec![(0, [0.0, 0.0, 1.0, 1.0])],
            Self::Both => vec![(0, [0.0, 0.0, 0.5, 1.0]), (1, [0.5, 0.0, 0.5, 1.0])],
            Self::Right => vec![(1, [0.0, 0.0, 1.0, 1.0])],
        }
    }
}

fn viewport_aspect(surface_size: [u32; 2], viewport: [f32; 4]) -> f32 {
    let width = surface_size[0].max(1) as f32 * viewport[2];
    let height = surface_size[1].max(1) as f32 * viewport[3];
    width / height.max(f32::EPSILON)
}

/// Owns the device state, render graph, and frame timing used by simulation nodes.
pub struct RenderContext {
    device: wgpu::Device,
    queue: wgpu::Queue,
    size: [u32; 2],
    output_format: wgpu::TextureFormat,
    asset_loader: Arc<dyn AssetLoader>,

    pub flows: Vec<Flow>,
    active_flows: RefCell<Vec<bool>>,
    last_render_instant: Cell<Instant>,
    pending_changes: Cell<NodeChanges>,
}

impl RenderContext {
    pub fn new(
        size: [u32; 2],
        flow_count: usize,
        device: wgpu::Device,
        queue: wgpu::Queue,
        output_format: wgpu::TextureFormat,
    ) -> Self {
        let mut flows = Vec::new();
        flows.resize_with(flow_count, Flow::new);

        Self {
            device,
            queue,
            size,
            output_format,
            asset_loader: Arc::new(|id: &AssetId| {
                std::fs::read(id.raw())
                    .map(std::io::Cursor::new)
                    .map_err(|err| format!("Cannot read asset '{}': {err}", id))
            }),
            flows,
            active_flows: RefCell::new(vec![true; flow_count]),
            last_render_instant: Cell::new(Instant::now()),
            pending_changes: Cell::new(NodeChanges::OUTPUT),
        }
    }

    pub fn set_asset_loader(&mut self, loader: impl AssetLoader + 'static) {
        self.asset_loader = Arc::new(loader);
    }

    pub fn load_asset(&self, id: &AssetId) -> Result<std::io::Cursor<Vec<u8>>, String> {
        self.asset_loader.load(id)
    }

    pub fn add_node(&mut self, node: Box<dyn Node>, flow_index: usize) {
        self.flows[flow_index].add_node(node);
    }

    pub fn replace_node(&self, index: usize, node: Box<dyn Node>, flow_index: usize) {
        self.flows[flow_index].replace_node(index, node);
    }

    pub fn resize(&mut self, new_size: [u32; 2]) {
        assert!(new_size[0] > 0 && new_size[1] > 0, "Non-positive size");
        self.size = [new_size[0], new_size[1]];
        self.apply_changes(NodeChanges::SLOTS);
    }

    pub fn delta_t(&self) -> f32 {
        self.last_render_instant.get().elapsed().as_micros() as f32
    }

    pub fn nodes_lens(&self) -> Vec<usize> {
        self.flows.iter().map(|flow| flow.nodes_len()).collect()
    }

    pub fn negociate_slots(&self) {
        for flow in self.flows.iter() {
            flow.negociate_slots(self);
        }
        self.apply_changes(NodeChanges::SLOTS);
    }

    pub fn apply_changes(&self, changes: NodeChanges) {
        self.pending_changes
            .set((self.pending_changes.get() | changes).normalized());
    }

    pub fn take_changes(&self) -> NodeChanges {
        let changes = self.pending_changes.get().normalized();
        self.pending_changes.set(NodeChanges::empty());
        changes
    }

    pub fn pending_changes(&self) -> NodeChanges {
        self.pending_changes.get().normalized()
    }

    pub fn device(&self) -> &wgpu::Device {
        &self.device
    }

    pub fn queue(&self) -> &wgpu::Queue {
        &self.queue
    }

    pub fn output_format(&self) -> wgpu::TextureFormat {
        self.output_format
    }

    pub fn width(&self) -> u32 {
        self.size[0]
    }

    pub fn height(&self) -> u32 {
        self.size[1]
    }

    pub fn render(&self, encoder: &mut wgpu::CommandEncoder, render_texture: &RenderTexture) {
        self.flows
            .iter()
            .enumerate()
            .filter(|(index, _)| self.flow_is_active(*index))
            .for_each(|(_, flow)| flow.render(self, encoder, render_texture));
    }

    pub fn render_flow(
        &self,
        flow_index: usize,
        encoder: &mut wgpu::CommandEncoder,
        render_texture: &RenderTexture,
    ) {
        self.flows[flow_index].render(self, encoder, render_texture);
    }

    pub fn post_render(&self) {
        self.flows
            .iter()
            .enumerate()
            .filter(|(index, _)| self.flow_is_active(*index))
            .for_each(|(_, flow)| flow.post_render(self));
        self.last_render_instant.replace(Instant::now());
    }

    pub fn flow_is_active(&self, flow_index: usize) -> bool {
        self.active_flows.borrow()[flow_index]
    }

    pub fn set_eye_mode(&self, mode: EyeMode) {
        assert!(self.flows.len() >= 2, "non-XR eye modes require two flows");
        let viewports = mode.flow_viewports();
        let mut active = self.active_flows.borrow_mut();
        active.fill(false);
        for (flow_index, viewport) in viewports {
            active[flow_index] = true;
            self.flows[flow_index].eye_mut().proj = cgmath::perspective(
                cgmath::Deg(70.0),
                viewport_aspect(self.size, viewport),
                0.05,
                1000.0,
            );
            self.flows[flow_index].try_with_unique_node_mut::<Display, _>(|display| {
                display.set_viewport(ViewPort {
                    x: viewport[0],
                    y: viewport[1],
                    width: viewport[2],
                    height: viewport[3],
                    absolute_viewport: false,
                });
            });
        }
        self.apply_changes(NodeChanges::OUTPUT);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn non_xr_eye_modes_have_deterministic_flow_viewports() {
        assert_eq!(
            EyeMode::Left.flow_viewports(),
            vec![(0, [0.0, 0.0, 1.0, 1.0])]
        );
        assert_eq!(
            EyeMode::Both.flow_viewports(),
            vec![(0, [0.0, 0.0, 0.5, 1.0]), (1, [0.5, 0.0, 0.5, 1.0]),]
        );
        assert_eq!(
            EyeMode::Right.flow_viewports(),
            vec![(1, [0.0, 0.0, 1.0, 1.0])]
        );
        assert_eq!(
            viewport_aspect([1280, 720], [0.0, 0.0, 1.0, 1.0]),
            16.0 / 9.0
        );
        assert_eq!(
            viewport_aspect([1280, 720], [0.0, 0.0, 0.5, 1.0]),
            8.0 / 9.0
        );
    }
}
