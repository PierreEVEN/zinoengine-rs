mod registry;
pub mod render_pass;

use registry::{ResourceData, ResourceHandle, ResourceRegistry};
use render_pass::{
    RenderPass, RenderPassBuilder, RenderPassExecutor, RenderPassType, TypedRenderPassExecutor,
};
use std::collections::HashMap;
use std::mem;
use std::mem::MaybeUninit;
use std::sync::Arc;
use ze_core::color::Color4f32;
use ze_gfx::backend::*;
use ze_gfx::PixelFormat;

#[derive(Clone)]
pub struct FrameGraphTextureDesc {
    pub format: PixelFormat,
    pub width: u32,
    pub height: u32,
}

pub struct FrameGraph<'a> {
    device: Arc<dyn Device>,
    resource_registry: ResourceRegistry,
    passes: Vec<RenderPass<'a>>,
}

impl<'a> FrameGraph<'a> {
    pub fn new(device: Arc<dyn Device>) -> Self {
        Self {
            device,
            resource_registry: ResourceRegistry::default(),
            passes: vec![],
        }
    }

    pub fn create_texture(&mut self, name: &str, desc: FrameGraphTextureDesc) -> ResourceHandle {
        self.resource_registry.create_texture(name, desc)
    }

    pub fn create_proxy(&mut self, handle: ResourceHandle) -> ResourceHandle {
        self.resource_registry.create_proxy(handle)
    }

    pub fn import_external_texture(&mut self, texture: Arc<Texture>, name: &str) -> ResourceHandle {
        assert!(texture
            .desc
            .usage_flags
            .contains(TextureUsageFlagBits::RenderTarget));

        let desc = FrameGraphTextureDesc {
            format: texture.desc.format,
            width: texture.desc.width,
            height: texture.desc.height,
        };

        let handle = self.resource_registry.create_texture(name, desc);
        self.resource_registry.resource_mut(handle).external = true;
        self.resource_registry.texture_mut(handle).resource = Some(texture);
        handle
    }

    pub fn add_pass<T, S, E>(&mut self, name: &str, ty: RenderPassType, setup: S, exec: E)
    where
        T: 'static,
        S: FnOnce(&mut RenderPassBuilder) -> T,
        E: FnMut(&CompiledFrameGraph, &T, &mut CommandList) + 'a,
    {
        assert!(
            !self.passes.iter().any(|pass| pass.name() == name),
            "Pass already exists"
        );

        let render_pass = {
            let mut builder = RenderPassBuilder::new(self);
            let data = setup(&mut builder);

            RenderPass {
                name: name.to_string(),
                ty,
                executor: Box::new(TypedRenderPassExecutor {
                    data,
                    func: Box::new(exec),
                }),
                reads: builder.reads,
                writes: builder.writes,
                writes_clear_color: builder.writes_clear_color,
                depth_stencil_input: builder.depth_stencil_input,
                depth_stencil_output: builder.depth_stencil_output,
                depth_stencil_clear_value: builder.depth_stencil_clear_value,
            }
        };
        self.passes.push(render_pass);
    }
}

/// Compiled [`FrameGraph`]
pub struct CompiledFrameGraph<'a> {
    device: Arc<dyn Device>,
    resource_registry: ResourceRegistry,
    passes: Vec<CompiledPass<'a>>,
    textures: Vec<CompiledTexture>,
    handle_to_compiled_texture: HashMap<ResourceHandle, usize>,
    rtvs: HashMap<ResourceHandle, RenderTargetView>,
    dsvs: HashMap<ResourceHandle, DepthStencilView>,
}

impl<'a> CompiledFrameGraph<'a> {
    fn new(
        device: Arc<dyn Device>,
        resource_registry: ResourceRegistry,
        passes: Vec<CompiledPass<'a>>,
        textures: Vec<CompiledTexture>,
    ) -> Self {
        let handle_to_compiled_texture = textures
            .iter()
            .enumerate()
            .map(|(i, h)| (h.handle, i))
            .collect();

        Self {
            device,
            resource_registry,
            passes,
            textures,
            handle_to_compiled_texture,
            rtvs: Default::default(),
            dsvs: Default::default(),
        }
    }

    pub fn execute(&mut self, cmd_list: &mut CommandList) {
        self.device.cmd_debug_begin_event(
            cmd_list,
            "Render Graph",
            Color4f32::new(0.75, 0.3, 0.15, 1.0),
        );

        let mut passes = mem::take(&mut self.passes);
        for pass in &mut passes {
            self.device.cmd_debug_begin_event(
                cmd_list,
                &pass.name,
                Color4f32::new(0.3, 0.75, 0.15, 1.0),
            );

            self.prepare_pass_resources(pass);

            // Apply invalidate barriers
            if !pass.invalidate_barriers.is_empty() {
                let mut barriers = Vec::with_capacity(pass.invalidate_barriers.len());
                for invalidate in &pass.invalidate_barriers {
                    barriers.push(ResourceBarrier::Transition(ResourceTransitionBarrier {
                        resource: ResourceTransitionBarrierResource::Texture(
                            self.resource_registry
                                .texture(invalidate.resource)
                                .resource
                                .as_ref()
                                .unwrap(),
                        ),
                        source_state: invalidate.src_state,
                        dest_state: invalidate.dst_state,
                    }));
                }

                self.device.cmd_resource_barrier(cmd_list, &barriers);
            }

            let rtvs = pass
                .render_targets
                .iter()
                .map(|rt| RenderPassRenderTarget {
                    render_target_view: &self.rtvs[&rt.texture],
                    load_mode: rt.load_mode,
                    store_mode: rt.store_mode,
                    clear_value: rt.clear_value,
                })
                .collect::<Vec<_>>();

            let dsv = pass
                .depth_stencil
                .as_ref()
                .map(|rt| RenderPassDepthStencil {
                    depth_stencil_view: &self.dsvs[&rt.texture],
                    load_mode: rt.load_mode,
                    store_mode: rt.store_mode,
                    clear_value: rt.clear_value,
                });

            let render_pass_desc = RenderPassDesc {
                render_targets: &rtvs,
                depth_stencil: dsv,
            };

            self.device
                .cmd_begin_render_pass(cmd_list, &render_pass_desc);
            pass.executor.execute(self, cmd_list);
            self.device.cmd_end_render_pass(cmd_list);

            // Apply flush barriers
            if !pass.flush_barriers.is_empty() {
                let mut barriers = Vec::with_capacity(pass.flush_barriers.len());
                for flush in &pass.flush_barriers {
                    barriers.push(ResourceBarrier::Transition(ResourceTransitionBarrier {
                        resource: ResourceTransitionBarrierResource::Texture(
                            self.resource_registry
                                .texture(flush.resource)
                                .resource
                                .as_ref()
                                .unwrap(),
                        ),
                        source_state: flush.src_state,
                        dest_state: flush.dst_state,
                    }));
                }

                self.device.cmd_resource_barrier(cmd_list, &barriers);
            }

            self.device.cmd_debug_end_event(cmd_list);
        }

        self.device.cmd_debug_end_event(cmd_list);
        self.passes = passes;
    }

    pub fn texture(&mut self, handle: ResourceHandle) -> &Arc<Texture> {
        let texture = self.resource_registry.texture(handle);
        texture.resource.as_ref().unwrap()
    }

    fn prepare_pass_resources(&mut self, pass: &mut CompiledPass<'a>) {
        for handle in pass
            .render_targets
            .iter()
            .map(|rt| &rt.texture)
            .chain(pass.depth_stencil.iter().map(|rt| &rt.texture))
        {
            let compiled_texture = &self.textures[self.handle_to_compiled_texture[handle]];
            let resource = self.resource_registry.resource(*handle);
            let texture = self.resource_registry.texture(*handle);
            if texture.resource.is_none() {
                let object = Arc::new(
                    self.device
                        .create_texture(
                            &TextureDesc {
                                width: compiled_texture.width,
                                height: compiled_texture.height,
                                depth: 1,
                                mip_levels: 1,
                                format: compiled_texture.format,
                                sample_desc: Default::default(),
                                usage_flags: compiled_texture.usage,
                                memory_desc: MemoryDesc {
                                    memory_location: MemoryLocation::GpuOnly,
                                    memory_flags: Default::default(),
                                },
                            },
                            Some(self.device.transient_memory_pool()),
                            &resource.name,
                        )
                        .expect("Failed to create texture"),
                );

                let texture = self.resource_registry.texture_mut(*handle);
                texture.resource = Some(object.clone());
            }
        }

        for rt in &pass.render_targets {
            #[allow(clippy::map_entry)]
            if !self.rtvs.contains_key(&rt.texture) {
                let texture = self.texture(rt.texture).clone();
                let format = texture.desc.format;
                let rtv = self
                    .device
                    .create_render_target_view(&RenderTargetViewDesc {
                        resource: texture,
                        format,
                        ty: RenderTargetViewType::Texture2D(Texture2DRTV { mip_level: 0 }),
                    })
                    .unwrap();
                self.rtvs.insert(rt.texture, rtv);
            }
        }

        if let Some(ds) = &pass.depth_stencil {
            #[allow(clippy::map_entry)]
            if !self.dsvs.contains_key(&ds.texture) {
                let texture = self.texture(ds.texture).clone();
                let format = texture.desc.format;
                let dsv = self
                    .device
                    .create_depth_stencil_view(&DepthStencilViewDesc {
                        resource: texture,
                        format,
                        ty: DepthStencilViewType::Texture2D(Texture2DDSV { mip_level: 0 }),
                    })
                    .unwrap();
                self.dsvs.insert(ds.texture, dsv);
            }
        }
    }
}

// Compilation implementation
struct CompiledPassRenderTarget {
    pub texture: ResourceHandle,
    pub load_mode: RenderPassTextureLoadMode,
    pub store_mode: RenderPassTextureStoreMode,
    pub clear_value: ClearValue,
}

struct Barrier {
    resource: ResourceHandle,
    src_state: ResourceState,
    dst_state: ResourceState,
}

struct CompiledPass<'a> {
    name: String,
    invalidate_barriers: Vec<Barrier>,
    flush_barriers: Vec<Barrier>,
    render_targets: Vec<CompiledPassRenderTarget>,
    depth_stencil: Option<CompiledPassRenderTarget>,
    writes: Vec<ResourceHandle>,
    executor: Box<dyn RenderPassExecutor<'a>>,
}

/// Data used while compiling a render graph
struct CompiledTexture {
    width: u32,
    height: u32,
    format: PixelFormat,
    usage: TextureUsageFlags,
    handle: ResourceHandle,
    alias_with: Option<usize>,
}

struct CompilationData<'a> {
    backbuffer: ResourceHandle,
    ordered_pass_list: Vec<usize>,
    handle_to_compiled_texture_idx: HashMap<ResourceHandle, usize>,
    textures: Vec<CompiledTexture>,
    free_texture_pool: Vec<usize>,
    compiled_passes: Vec<CompiledPass<'a>>,
}

impl<'a> FrameGraph<'a> {
    pub fn compile(mut self, backbuffer: ResourceHandle) -> CompiledFrameGraph<'a> {
        let mut compilation_data = CompilationData {
            backbuffer: self.resource_registry.resolve_handle(backbuffer),
            ordered_pass_list: Vec::with_capacity(self.passes.len()),
            handle_to_compiled_texture_idx: Default::default(),
            textures: Default::default(),
            free_texture_pool: vec![],
            compiled_passes: vec![],
        };

        // Acquire all passes writing directly to the backbuffer and add them to the final pass list
        self.passes
            .iter()
            .enumerate()
            .filter(|(_, pass)| pass.writes.iter().any(|output| output == &backbuffer))
            .for_each(|(i, _)| {
                compilation_data.ordered_pass_list.push(i);
            });

        // Now traverse all passes that writes to resources needed by the passes writing to the backbuffer
        // This will cull unused passes
        {
            let mut pass_queue = compilation_data.ordered_pass_list.clone();
            while let Some(pass) = pass_queue.pop() {
                let pass = &self.passes[pass];
                for &input in &pass.reads {
                    for (i, pass) in self.passes.iter().enumerate() {
                        if pass.writes.iter().any(|&output| output == input)
                            && !compilation_data.ordered_pass_list.contains(&i)
                        {
                            compilation_data.ordered_pass_list.push(i);
                            pass_queue.push(i);
                        }
                    }
                }
            }
        }

        compilation_data.ordered_pass_list.dedup();
        compilation_data.ordered_pass_list.reverse();

        // Ordered pass list is now in the correct order
        self.build_physical_textures(&mut compilation_data);
        self.build_physical_passes(&mut compilation_data);
        self.build_barriers(&mut compilation_data);

        CompiledFrameGraph::new(
            self.device,
            self.resource_registry,
            compilation_data.compiled_passes,
            compilation_data.textures,
        )
    }

    fn build_physical_textures(&mut self, compilation_data: &mut CompilationData) {
        let ordered_pass_list = compilation_data.ordered_pass_list.clone();
        for pass_idx in ordered_pass_list {
            // Set textures last use
            let pass = &self.passes[pass_idx];
            for &texture in pass.iter_resources() {
                if !self.resource_registry.is_texture(texture) {
                    continue;
                }

                self.resource_registry.resource_mut(texture).last_pass_use = Some(pass_idx);
            }

            // Collect texture usages
            for &read in &pass.reads {
                let texture = self.add_physical_texture(compilation_data, read);
                texture.usage |= TextureUsageFlagBits::Sampled;
            }

            for &write in &pass.writes {
                let texture = self.add_physical_texture(compilation_data, write);
                texture.usage |= TextureUsageFlagBits::RenderTarget;
            }

            if let Some(depth_stencil_input) = pass.depth_stencil_input {
                let texture = self.add_physical_texture(compilation_data, depth_stencil_input);
                texture.usage |= TextureUsageFlagBits::DepthStencil;
            } else if let Some(depth_stencil_output) = pass.depth_stencil_output {
                let texture = self.add_physical_texture(compilation_data, depth_stencil_output);
                texture.usage |= TextureUsageFlagBits::DepthStencil;
            }

            // If at this pass, any textures are not anymore used, we can push them to the free texture pool
            for &texture in pass.iter_resources() {
                let texture = self.resource_registry.resolve_handle(texture);

                if self.resource_registry.is_texture(texture)
                    && self.resource_registry.resource(texture).last_pass_use == Some(pass_idx)
                {
                    compilation_data
                        .free_texture_pool
                        .push(compilation_data.handle_to_compiled_texture_idx[&texture]);
                }
            }
        }
    }

    fn build_physical_passes(&mut self, compilation_data: &mut CompilationData<'a>) {
        let ordered_pass_list = compilation_data.ordered_pass_list.clone();
        let mut passes = mem::take(&mut self.passes)
            .into_iter()
            .map(MaybeUninit::new)
            .collect::<Vec<_>>();

        for pass_idx in ordered_pass_list {
            let pass =
                unsafe { mem::replace(&mut passes[pass_idx], MaybeUninit::uninit()).assume_init() };
            let mut render_targets = Vec::with_capacity(MAX_RENDER_PASS_RENDER_TARGET_COUNT);
            let mut depth_stencil = None;
            for (i, &output) in pass.writes.iter().enumerate() {
                let output = self.resource_registry.resolve_handle(output);
                if self.resource_registry.is_texture(output) {
                    let clear_value = &pass.writes_clear_color[i];
                    let load_mode = {
                        if pass.reads.contains(&output) {
                            RenderPassTextureLoadMode::Preserve
                        } else if clear_value.is_some() {
                            RenderPassTextureLoadMode::Clear
                        } else {
                            RenderPassTextureLoadMode::Discard
                        }
                    };
                    render_targets.push(CompiledPassRenderTarget {
                        texture: output,
                        load_mode,
                        store_mode: RenderPassTextureStoreMode::Preserve,
                        clear_value: clear_value.unwrap_or(ClearValue::Color([0.0, 0.0, 0.0, 0.0])),
                    });
                }
            }

            if let Some(depth_stencil_input) = pass.depth_stencil_input {
                let clear_value = pass.depth_stencil_clear_value.unwrap();
                depth_stencil = Some(CompiledPassRenderTarget {
                    texture: depth_stencil_input,
                    load_mode: RenderPassTextureLoadMode::Preserve,
                    store_mode: RenderPassTextureStoreMode::Preserve,
                    clear_value,
                });
            } else if let Some(depth_stencil_output) = pass.depth_stencil_output {
                let clear_value = pass.depth_stencil_clear_value.unwrap();
                depth_stencil = Some(CompiledPassRenderTarget {
                    texture: depth_stencil_output,
                    load_mode: RenderPassTextureLoadMode::Clear,
                    store_mode: RenderPassTextureStoreMode::Preserve,
                    clear_value,
                });
            }

            compilation_data.compiled_passes.push(CompiledPass {
                name: pass.name,
                invalidate_barriers: vec![],
                flush_barriers: vec![],
                render_targets,
                depth_stencil,
                writes: pass.writes,
                executor: pass.executor,
            });
        }
    }

    fn build_barriers(&self, compilation_data: &mut CompilationData) {
        // The algorithm is quite simple:
        // - We traverse each render pass, making a barrier depending on the requested resource state and the current resource state
        //
        // Special cases:
        // - Backbuffer initial state is considered Present
        // - Backbuffer final state will be Present

        let mut resource_states = Vec::with_capacity(self.resource_registry.resources().len());
        for i in 0..self.resource_registry.resources().len() {
            resource_states.push(if i == compilation_data.backbuffer.0 {
                ResourceState::Present
            } else {
                ResourceState::Common
            });
        }

        for compiled_pass in &mut compilation_data.compiled_passes {
            for &color_output in &compiled_pass.writes {
                let color_output = self.resource_registry.resolve_handle(color_output);
                let src_state = resource_states[color_output.0];
                if src_state != ResourceState::RenderTargetWrite {
                    compiled_pass.invalidate_barriers.push(Barrier {
                        resource: color_output,
                        src_state,
                        dst_state: ResourceState::RenderTargetWrite,
                    });

                    resource_states[color_output.0] = ResourceState::RenderTargetWrite;
                }
            }
        }

        let last_idx = compilation_data.compiled_passes.len() - 1;
        compilation_data.compiled_passes[last_idx]
            .flush_barriers
            .push(Barrier {
                resource: compilation_data.backbuffer,
                src_state: resource_states[compilation_data.backbuffer.0],
                dst_state: ResourceState::Present,
            });
    }

    fn add_physical_texture<'b>(
        &self,
        compilation_data: &'b mut CompilationData,
        handle: ResourceHandle,
    ) -> &'b mut CompiledTexture {
        if let ResourceData::Proxy(texture) = &self.resource_registry.resource(handle).data {
            self.add_physical_texture(compilation_data, *texture)
        } else {
            let texture = self.resource_registry.texture(handle);

            // Fetch the free pool to find a texture that can be reused
            let reusable_texture = {
                if !self.resource_registry.is_external(handle) {
                    let reusable_texture_idx =
                        compilation_data
                            .free_texture_pool
                            .iter()
                            .position(|&free_texture_idx| {
                                let free_texture = &compilation_data.textures[free_texture_idx];
                                let can_contain_texture = free_texture
                                    .format
                                    .texture_size_in_bytes(free_texture.width, free_texture.height)
                                    >= texture.desc.format.texture_size_in_bytes(
                                        texture.desc.width,
                                        texture.desc.height,
                                    );

                                can_contain_texture
                                    && !self.resource_registry.is_external(free_texture.handle)
                            });

                    reusable_texture_idx.map(|idx| compilation_data.free_texture_pool.remove(idx))
                } else {
                    None
                }
            };

            assert!(reusable_texture.is_none(), "aliasing not implemented yet");

            let idx = compilation_data
                .handle_to_compiled_texture_idx
                .entry(handle)
                .or_insert_with(|| {
                    compilation_data.textures.push(CompiledTexture {
                        width: texture.desc.width,
                        height: texture.desc.height,
                        format: texture.desc.format,
                        usage: TextureUsageFlags::empty(),
                        handle,
                        alias_with: reusable_texture,
                    });
                    compilation_data.textures.len() - 1
                });

            &mut compilation_data.textures[*idx]
        }
    }
}
