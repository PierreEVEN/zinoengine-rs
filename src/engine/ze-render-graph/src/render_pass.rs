use crate::registry::ResourceHandle;
use crate::{CompiledFrameGraph, FrameGraph, FrameGraphTextureDesc};
use ze_gfx::backend::{ClearValue, CommandList};

pub enum RenderPassType {
    Graphics,
    Compute,
}

pub(crate) trait RenderPassExecutor<'graph>: 'graph {
    fn execute(&mut self, render_graph: &CompiledFrameGraph, command_list: &mut CommandList);
}

pub(crate) struct TypedRenderPassExecutor<'graph, T> {
    pub data: T,
    pub func: Box<dyn FnMut(&CompiledFrameGraph, &T, &mut CommandList) + 'graph>,
}

impl<'graph, T: 'static> RenderPassExecutor<'graph> for TypedRenderPassExecutor<'graph, T> {
    fn execute(&mut self, render_graph: &CompiledFrameGraph, command_list: &mut CommandList) {
        (self.func)(render_graph, &self.data, command_list);
    }
}

pub(crate) struct RenderPass<'graph> {
    pub name: String,
    pub ty: RenderPassType,
    pub executor: Box<dyn RenderPassExecutor<'graph>>,
    pub reads: Vec<ResourceHandle>,
    pub writes: Vec<ResourceHandle>,
    pub writes_clear_color: Vec<Option<ClearValue>>,
    pub depth_stencil_input: Option<ResourceHandle>,
    pub depth_stencil_output: Option<ResourceHandle>,
    pub depth_stencil_clear_value: Option<ClearValue>,
}

impl<'graph> RenderPass<'graph> {
    pub fn iter_resources(&self) -> impl Iterator<Item = &ResourceHandle> {
        self.reads
            .iter()
            .chain(self.writes.iter())
            .chain(self.depth_stencil_input.iter())
            .chain(self.depth_stencil_output.iter())
    }

    pub fn name(&self) -> &str {
        &self.name
    }
}

pub struct RenderPassBuilder<'a, 'b> {
    graph: &'b mut FrameGraph<'a>,
    pub(crate) reads: Vec<ResourceHandle>,
    pub(crate) writes: Vec<ResourceHandle>,
    pub(crate) writes_clear_color: Vec<Option<ClearValue>>,
    pub(crate) depth_stencil_input: Option<ResourceHandle>,
    pub(crate) depth_stencil_output: Option<ResourceHandle>,
    pub(crate) depth_stencil_clear_value: Option<ClearValue>,
}

impl<'a, 'b> RenderPassBuilder<'a, 'b> {
    pub fn new(graph: &'b mut FrameGraph<'a>) -> Self {
        Self {
            graph,
            reads: vec![],
            writes: vec![],
            writes_clear_color: vec![],
            depth_stencil_input: None,
            depth_stencil_output: None,
            depth_stencil_clear_value: None,
        }
    }

    pub fn create_texture(&mut self, name: &str, desc: FrameGraphTextureDesc) -> ResourceHandle {
        self.graph.create_texture(name, desc)
    }

    #[must_use = "returns a updated resource handle that must be used to reference this resource in the future"]
    pub fn read(&mut self, resource: ResourceHandle) -> ResourceHandle {
        if let Some(i) = self.writes.iter().position(|r| *r == resource) {
            self.writes.remove(i);
            self.writes.push(resource);
        }
        self.reads.push(resource);
        resource
    }

    #[must_use = "returns a updated resource handle that must be used to reference this resource in the future"]
    pub fn write(&mut self, resource: ResourceHandle) -> ResourceHandle {
        let resource = if self.reads.contains(&resource) {
            self.graph.create_proxy(resource)
        } else {
            resource
        };
        self.writes.push(resource);
        self.writes_clear_color.push(None);
        resource
    }

    pub fn set_clear_color(&mut self, resource: ResourceHandle, clear_color: ClearValue) {
        if let Some(i) = self.writes.iter().position(|r| *r == resource) {
            self.writes_clear_color[i] = Some(clear_color);
        }
    }

    pub fn set_depth_stencil_output(&mut self, resource: ResourceHandle, clear_color: ClearValue) {
        self.depth_stencil_output = Some(resource);
        self.depth_stencil_clear_value = Some(clear_color);
    }
}
