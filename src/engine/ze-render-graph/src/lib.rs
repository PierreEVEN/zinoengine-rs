use crate::registry::{PhysicalResourceHandle, PhysicalResourceRegistry};
use std::collections::HashMap;
use std::ptr;
use std::sync::Arc;
use ze_core::color::Color4f32;
use ze_gfx::backend::*;
use ze_gfx::PixelFormat;

#[derive(Copy, Clone, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct RenderPassHandle(usize);

#[derive(Copy, Clone, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct ResourceHandle(usize);

#[derive(PartialEq, Eq)]
pub struct TextureInfo {
    pub format: PixelFormat,
    pub width: u32,
    pub height: u32,
    pub usage_flags: TextureUsageFlags,
}

impl Default for TextureInfo {
    fn default() -> Self {
        Self {
            format: PixelFormat::Unknown,
            width: 0,
            height: 0,
            usage_flags: TextureUsageFlags::empty(),
        }
    }
}

#[derive(PartialEq, Eq)]
enum ResourceData {
    None,
    Texture(TextureInfo),
}

struct Resource {
    name: String,
    handle: ResourceHandle,
    physical_handle: Option<PhysicalResourceHandle>,
    pass_reads: Vec<RenderPassHandle>,
    pass_writes: Vec<RenderPassHandle>,
    data: ResourceData,
}

impl Resource {
    fn new(name: &str, handle: ResourceHandle) -> Self {
        Self {
            name: name.to_string(),
            handle,
            physical_handle: None,
            pass_reads: vec![],
            pass_writes: vec![],
            data: ResourceData::None,
        }
    }
}

struct Barrier {
    resource: ResourceHandle,
    src_state: ResourceState,
    dst_state: ResourceState,
}

struct RenderPass<'a> {
    name: String,
    exec_fn: Box<dyn FnMut(&Arc<dyn Device>, &mut CommandList) + 'a>,
    dependencies: Vec<RenderPassHandle>,
    color_outputs: Vec<ResourceHandle>,
    invalidate_barriers: Vec<Barrier>,
    flush_barriers: Vec<Barrier>,
}

impl<'a> RenderPass<'a> {
    fn new(name: &str, exec_fn: Box<dyn FnMut(&Arc<dyn Device>, &mut CommandList) + 'a>) -> Self {
        Self {
            name: name.to_string(),
            exec_fn,
            color_outputs: vec![],
            dependencies: vec![],
            invalidate_barriers: vec![],
            flush_barriers: vec![],
        }
    }
}

pub struct RenderGraph<'a> {
    device: Arc<dyn Device>,
    registry: &'a mut PhysicalResourceRegistry,
    resources: Vec<Resource>,
    render_passes: Vec<RenderPass<'a>>,
    resource_name_map: HashMap<String, ResourceHandle>,
    backbuffer_resource: Option<ResourceHandle>,
    final_pass_list: Vec<RenderPassHandle>,
}

impl<'a> RenderGraph<'a> {
    pub fn new(device: Arc<dyn Device>, registry: &'a mut PhysicalResourceRegistry) -> Self {
        Self {
            device,
            registry,
            resources: vec![],
            render_passes: vec![],
            resource_name_map: Default::default(),
            backbuffer_resource: None,
            final_pass_list: vec![],
        }
    }

    pub fn add_graphics_pass(
        &mut self,
        name: &str,
        mut setup_fn: impl FnMut(&mut Self, RenderPassHandle),
        exec_fn: impl FnMut(&Arc<dyn Device>, &mut CommandList) + 'a,
    ) -> RenderPassHandle {
        let exec_fn = Box::new(exec_fn);
        self.render_passes.push(RenderPass::new(name, exec_fn));
        let handle = RenderPassHandle(self.render_passes.len() - 1);
        setup_fn(self, handle);
        handle
    }

    fn get_or_create_resource(&mut self, name: &str) -> &mut Resource {
        if let Some(resource) = self.resource_name_map.get(name) {
            &mut self.resources[resource.0]
        } else {
            let handle = ResourceHandle(self.resources.len());
            self.resources.push(Resource::new(name, handle));
            self.resource_name_map.insert(name.to_string(), handle);
            &mut self.resources[handle.0]
        }
    }

    fn set_backbuffer(&mut self, name: &str) {
        let rtv = self
            .registry
            .get_render_target_view(
                self.registry
                    .get_handle_from_name(name)
                    .expect("Backbuffer not present in the registry!"),
            )
            .unwrap();

        let info = TextureInfo {
            format: rtv.desc.resource.desc.format,
            width: rtv.desc.resource.desc.width,
            height: rtv.desc.resource.desc.height,
            usage_flags: rtv.desc.resource.desc.usage_flags,
        };

        let resource = self.get_or_create_resource(name);
        debug_assert!(matches!(resource.data, ResourceData::Texture(_)));

        resource.data = ResourceData::Texture(info);

        self.backbuffer_resource = Some(resource.handle);
    }

    pub fn compile(&mut self, backbuffer_name: &str) {
        self.set_backbuffer(backbuffer_name);
        assert!(self.backbuffer_resource.is_some());

        // The final ordered pass list
        let mut pass_list = vec![];

        let backbuffer_resource = self.backbuffer_resource.as_ref().unwrap();
        let backbuffer = &self.resources[backbuffer_resource.0];

        // List all render passes that directly write to the back buffer
        for pass in &backbuffer.pass_writes {
            pass_list.push(*pass);
        }

        // Traverse each pass dependencies to get a unordered list of all referenced render passes
        {
            let pass_list_copy = pass_list.clone();
            for pass in pass_list_copy {
                self.traverse_pass_dependencies(&mut pass_list, pass, 0);
            }
        }

        pass_list.reverse();
        pass_list.dedup();
        self.order_passes(&mut pass_list);
        self.final_pass_list = pass_list;
        self.build_physical_resources();
        self.build_barriers();
    }

    pub fn execute(mut self, command_list: &mut CommandList) {
        self.device.cmd_debug_begin_event(
            command_list,
            "Render Graph",
            Color4f32::new(0.75, 0.3, 0.15, 1.0),
        );

        for handle in &self.final_pass_list {
            let render_pass = &mut self.render_passes[handle.0];

            self.device.cmd_debug_begin_event(
                command_list,
                &render_pass.name,
                Color4f32::new(0.3, 0.75, 0.15, 1.0),
            );

            let mut render_targets = Vec::with_capacity(render_pass.color_outputs.len());
            for output in &render_pass.color_outputs {
                let resource = &self.resources[output.0];
                let physical_resource = resource.physical_handle.unwrap();
                let rtv = self
                    .registry
                    .get_render_target_view(physical_resource)
                    .unwrap();
                render_targets.push(RenderPassTexture {
                    render_target_view: rtv,
                    load_mode: RenderPassTextureLoadMode::Clear,
                    store_mode: RenderPassTextureStoreMode::Preserve,
                    clear_value: ClearValue::Color([0.0, 0.0, 0.0, 1.0]),
                });
            }

            // Apply invalidate barriers
            if !render_pass.invalidate_barriers.is_empty() {
                let mut barriers = Vec::with_capacity(render_pass.invalidate_barriers.len());
                for invalidate in &render_pass.invalidate_barriers {
                    let resource = &self.resources[invalidate.resource.0];
                    barriers.push(ResourceBarrier::Transition(ResourceTransitionBarrier {
                        resource: ResourceTransitionBarrierResource::Texture(
                            self.registry
                                .get_texture(resource.physical_handle.unwrap())
                                .unwrap(),
                        ),
                        source_state: invalidate.src_state,
                        dest_state: invalidate.dst_state,
                    }));
                }

                self.device.cmd_resource_barrier(command_list, &barriers);
            }

            self.device.cmd_begin_render_pass(
                command_list,
                &RenderPassDesc {
                    render_targets: &render_targets,
                    depth_stencil: None,
                },
            );
            (render_pass.exec_fn)(&self.device, command_list);

            self.device.cmd_end_render_pass(command_list);

            if !render_pass.flush_barriers.is_empty() {
                let mut barriers = Vec::with_capacity(render_pass.flush_barriers.len());
                for flush in &render_pass.flush_barriers {
                    let resource = &self.resources[flush.resource.0];
                    barriers.push(ResourceBarrier::Transition(ResourceTransitionBarrier {
                        resource: ResourceTransitionBarrierResource::Texture(
                            self.registry
                                .get_texture(resource.physical_handle.unwrap())
                                .unwrap(),
                        ),
                        source_state: flush.src_state,
                        dest_state: flush.dst_state,
                    }));
                }

                self.device.cmd_resource_barrier(command_list, &barriers);
            }

            self.device.cmd_debug_end_event(command_list);
        }

        self.device.cmd_debug_end_event(command_list);
    }

    fn build_physical_resources(&mut self) {
        for pass_handle in &self.final_pass_list {
            let pass = &self.render_passes[pass_handle.0];
            for output_handle in &pass.color_outputs {
                let mut resource = &mut self.resources[output_handle.0];
                if let ResourceData::Texture(info) = &resource.data {
                    resource.physical_handle =
                        Some(self.registry.get_or_create_texture(&resource.name, info))
                } else {
                    panic!(
                        "Resource {} must be a texture to be used as a color output for a pass!",
                        resource.name
                    );
                }
            }
        }
    }

    fn build_barriers(&mut self) {
        // The algorithm is quite simple:
        // - We traverse each render pass, making a barrier depending on the requested resource state and the current resource state
        //
        // Special cases:
        // - Backbuffer initial state is considered Present
        // - Backbuffer final state will be Present

        let mut resource_states = Vec::with_capacity(self.resources.len());
        for i in 0..self.resources.len() {
            resource_states.push(if i == self.backbuffer_resource.unwrap().0 {
                ResourceState::Present
            } else {
                ResourceState::Common
            });
        }

        for pass_handle in &self.final_pass_list {
            let pass = &mut self.render_passes[pass_handle.0];

            for color_output in &pass.color_outputs {
                pass.invalidate_barriers.push(Barrier {
                    resource: *color_output,
                    src_state: resource_states[color_output.0],
                    dst_state: ResourceState::RenderTargetWrite,
                });

                resource_states[color_output.0] = ResourceState::RenderTargetWrite;
            }
        }

        let backbuffer = self.backbuffer_resource.unwrap();
        self.render_passes[self.final_pass_list.last().unwrap().0]
            .flush_barriers
            .push(Barrier {
                resource: self.backbuffer_resource.unwrap(),
                src_state: resource_states[backbuffer.0],
                dst_state: ResourceState::Present,
            });
    }

    fn order_passes(&self, pass_list: &mut Vec<RenderPassHandle>) {
        let schedule = |index: usize,
                        passes: &mut Vec<RenderPassHandle>,
                        final_pass_list: &mut Vec<RenderPassHandle>| {
            final_pass_list.push(passes[index]);
            passes.copy_within(index + 1.., index);
            passes.pop();
        };

        let mut final_pass_list = Vec::with_capacity(pass_list.len());
        schedule(0, pass_list, &mut final_pass_list);

        while !pass_list.is_empty() {
            let mut pass_to_schedule = 0;
            for (i, _) in pass_list.iter().enumerate() {
                let mut candidate = true;
                for j in 0..i {
                    if self.depends_on_pass(&self.render_passes[i], &self.render_passes[j]) {
                        candidate = true;
                        break;
                    }
                }

                if !candidate {
                    continue;
                }

                pass_to_schedule = i;
            }

            schedule(pass_to_schedule, pass_list, &mut final_pass_list);
        }

        *pass_list = final_pass_list;
    }

    fn depends_on_pass(&self, a: &RenderPass, b: &RenderPass) -> bool {
        if ptr::eq(a as *const _, b as *const _) {
            return true;
        }

        for dependency in &a.dependencies {
            if self.depends_on_pass(&self.render_passes[dependency.0], b) {
                return true;
            }
        }

        false
    }

    fn traverse_pass_dependencies(
        &mut self,
        pass_list: &mut Vec<RenderPassHandle>,
        pass: RenderPassHandle,
        stack_depth: usize,
    ) {
        let render_pass = &self.render_passes[pass.0];

        // Collect pass dependencies and add them later on
        let pass_dependencies = vec![];

        // TODO: Collect attachment inputs & color inputs
        // See https://github.com/Zino2201/Prism/blob/main/src/engine/prism_gfx/src/render_graph.rs

        self.add_pass_recursive(pass_list, pass, &pass_dependencies, stack_depth);
    }

    fn add_pass_recursive(
        &mut self,
        pass_list: &mut Vec<RenderPassHandle>,
        pass: RenderPassHandle,
        written_passes: &Vec<RenderPassHandle>,
        stack_depth: usize,
    ) {
        assert!(stack_depth < self.render_passes.len());

        for write_pass in written_passes {
            if *write_pass != pass {
                self.render_passes[pass.0].dependencies.push(*write_pass);
                pass_list.push(*write_pass);
                self.traverse_pass_dependencies(pass_list, *write_pass, stack_depth + 1);
            }
        }
    }
}

// Render pass manipulation functions
impl<'a> RenderGraph<'a> {
    pub fn add_pass_color_output(
        &mut self,
        render_pass_handle: RenderPassHandle,
        name: &str,
        info: TextureInfo,
    ) -> ResourceHandle {
        let resource = self.get_or_create_resource(name);
        match &mut resource.data {
            ResourceData::None => resource.data = ResourceData::Texture(info),
            ResourceData::Texture(info) => {
                info.usage_flags.insert(TextureUsageFlagBits::RenderTarget);
            }
        }
        resource.pass_writes.push(render_pass_handle);

        let handle = resource.handle;
        let render_pass = &mut self.render_passes[render_pass_handle.0];
        render_pass.color_outputs.push(handle);
        handle
    }
}

pub mod registry;
