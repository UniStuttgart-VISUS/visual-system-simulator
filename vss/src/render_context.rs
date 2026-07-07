use crate::*;
use instant::Instant;
use std::cell::Cell;

/// Owns the device state, render graph, and frame timing used by simulation nodes.
pub struct RenderContext {
    device: wgpu::Device,
    queue: wgpu::Queue,
    size: [u32; 2],
    output_format: wgpu::TextureFormat,

    pub flows: Vec<Flow>,
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
            flows,
            last_render_instant: Cell::new(Instant::now()),
            pending_changes: Cell::new(NodeChanges::OUTPUT),
        }
    }

    pub fn add_node(&mut self, node: Box<dyn Node>, flow_index: usize) {
        self.flows[flow_index].add_node(node);
    }

    pub fn replace_node(&mut self, index: usize, node: Box<dyn Node>, flow_index: usize) {
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

    pub fn inspect(&self, inspector: &mut dyn Inspector) -> NodeChanges {
        let mut result = NodeChanges::empty();
        for (i, flow) in self.flows.iter().enumerate() {
            result |= inspector.flow(i, flow);
        }
        self.apply_changes(result);
        result.normalized()
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
            .for_each(|flow| flow.render(self, encoder, render_texture));
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
        self.flows.iter().for_each(|flow| flow.post_render(self));
        self.last_render_instant.replace(Instant::now());
    }
}
