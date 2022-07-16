use crate::{PixelFormat, SampleDesc, ShaderStageFlagBits};
use enumflags2::{bitflags, BitFlags};
use raw_window_handle::RawWindowHandle;
use std::any::Any;
use std::sync::Arc;
use ze_core::color::Color4f32;
use ze_core::maths::{RectI32, Vec2f32, Vec3i32, Vec4f32};

#[derive(Ord, PartialOrd, Eq, PartialEq, Debug)]
pub enum BackendError {
    Unsupported,
}

pub trait Backend: Send + Sync {
    fn create_device(&self) -> Result<Arc<dyn Device>, BackendError>;
}

#[derive(Ord, PartialOrd, Eq, PartialEq, Debug)]
pub enum DeviceError {
    Unknown,
    OutOfMemory,
    NoCompatibleMemoryTypeFound,
    InvalidParameters,
}

#[derive(Copy, Clone, Eq, PartialEq)]
pub enum RenderPassTextureLoadMode {
    Discard,
    Preserve,
    Clear,
}

#[derive(Copy, Clone, Eq, PartialEq)]
pub enum RenderPassTextureStoreMode {
    Discard,
    Preserve,
    Resolve,
}

pub enum ClearValue {
    Color([f32; 4]),
    DepthStencil((f32, u8)),
}

pub struct RenderPassTexture<'a> {
    pub render_target_view: &'a RenderTargetView,
    pub load_mode: RenderPassTextureLoadMode,
    pub store_mode: RenderPassTextureStoreMode,
    pub clear_value: ClearValue,
}

pub struct RenderPassDesc<'a> {
    pub render_targets: &'a [RenderPassTexture<'a>],
    pub depth_stencil: Option<RenderPassTexture<'a>>,
}

#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub enum QueueType {
    Graphics,
    Compute,
    Transfer,
}

#[derive(Eq, PartialEq, Copy, Clone, Debug)]
pub enum ResourceState {
    Common,
    IndexBufferRead,
    UnorderedAccessReadWrite,
    RenderTargetWrite,
    DepthRead,
    DepthWrite,
    ShaderRead,
    CopyRead,
    CopyWrite,
    Present,
}

pub enum ResourceTransitionBarrierResource<'a> {
    Buffer(&'a Buffer),
    Texture(&'a Texture),
}

pub struct ResourceTransitionBarrier<'a> {
    pub resource: ResourceTransitionBarrierResource<'a>,
    pub source_state: ResourceState,
    pub dest_state: ResourceState,
}

pub enum ResourceBarrier<'a> {
    Transition(ResourceTransitionBarrier<'a>),
}

// Pipeline states
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub enum PrimitiveTopology {
    Point,
    Line,
    Triangle,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct PipelineInputAssemblyState {
    pub primitive_topology: PrimitiveTopology,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub enum BlendFactor {
    Zero,
    One,
    SrcColor,
    OneMinusSrcColor,
    DstColor,
    OneMinusDstColor,
    SrcAlpha,
    OneMinusSrcAlpha,
    DstAlpha,
    OneMinusDstAlpha,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub enum BlendOp {
    Add,
    Subtract,
    ReverseSubtract,
    Min,
    Max,
}

pub struct PipelineRenderTargetBlendDesc {
    pub enable_blend: bool,
    pub src_color_blend_factor: BlendFactor,
    pub dst_color_blend_factor: BlendFactor,
    pub color_blend_op: BlendOp,
    pub src_alpha_blend_factor: BlendFactor,
    pub dst_alpha_blend_factor: BlendFactor,
    pub alpha_blend_op: BlendOp,
}

impl Default for PipelineRenderTargetBlendDesc {
    fn default() -> Self {
        Self {
            enable_blend: false,
            src_color_blend_factor: BlendFactor::Zero,
            dst_color_blend_factor: BlendFactor::Zero,
            color_blend_op: BlendOp::Add,
            src_alpha_blend_factor: BlendFactor::Zero,
            dst_alpha_blend_factor: BlendFactor::Zero,
            alpha_blend_op: BlendOp::Add,
        }
    }
}

#[derive(Default)]
pub struct PipelineBlendState {
    pub render_targets: [PipelineRenderTargetBlendDesc; MAX_RENDER_PASS_RENDER_TARGET_COUNT],
}

#[derive(Clone)]
pub struct PipelineShaderStage<'a> {
    pub stage: ShaderStageFlagBits,
    pub module: &'a ShaderModule,
}

pub struct BufferCopyRegion {
    pub src_offset_in_bytes: u64,
    pub dst_offset_in_bytes: u64,
    pub size_in_bytes: u64,
}

pub struct BufferToTextureCopyRegion {
    pub buffer_offset: u64,
    pub texture_subresource_index: u32,
    pub texture_subresource_layout: TextureSubresourceLayout,
    pub texture_subresource_width: u32,
    pub texture_subresource_height: u32,
    pub texture_subresource_depth: u32,
    pub texture_subresource_offset: Vec3i32,
}

pub enum IndexBufferFormat {
    Uint16,
    Uint32,
}

// Shader resource view

#[derive(Clone)]
pub struct BufferSRV {
    pub first_element_index: u32,
    pub element_count: u32,
    pub element_size_in_bytes: u32,
}

#[derive(Clone)]
pub struct Texture2DSRV {
    pub min_mip_level: u32,
    pub mip_levels: u32,
}

#[derive(Clone)]
pub enum ShaderResourceViewType {
    Buffer(BufferSRV),
    Texture2D(Texture2DSRV),
}

#[derive(Clone)]
pub enum ShaderResourceViewResource {
    Buffer(Arc<Buffer>),
    Texture(Arc<Texture>),
}

#[derive(Clone)]
pub struct ShaderResourceViewDesc {
    pub resource: ShaderResourceViewResource,
    pub format: PixelFormat,
    pub ty: ShaderResourceViewType,
}

// Render target view

#[derive(Clone)]
pub struct Texture2DRTV {
    pub mip_level: u32,
}

#[derive(Clone)]
pub enum RenderTargetViewType {
    Texture2D(Texture2DRTV),
}

#[derive(Clone)]
pub struct RenderTargetViewDesc {
    pub resource: Arc<Texture>,
    pub format: PixelFormat,
    pub ty: RenderTargetViewType,
}

pub struct Viewport {
    pub position: Vec2f32,
    pub size: Vec2f32,
    pub min_depth: f32,
    pub max_depth: f32,
}

#[derive(Copy, Clone)]
pub enum Filter {
    Nearest,
    Linear,
}

#[derive(Copy, Clone)]
pub enum TextureAddressMode {
    Repeat,
    Mirror,
    Clamp,
}

#[derive(Copy, Clone)]
pub enum CompareOp {
    Never,
    Less,
    Equal,
    LessEqual,
    Greater,
    NotEqual,
    GreaterEqual,
    Always,
}

#[derive(Clone)]
pub struct SamplerDesc {
    pub filter: Filter,
    pub address_u: TextureAddressMode,
    pub address_v: TextureAddressMode,
    pub address_w: TextureAddressMode,
    pub mip_lod_bias: f32,
    pub max_anisotropy: u32,
    pub compare_op: CompareOp,
    pub min_lod: f32,
    pub max_lod: f32,
}

impl Default for SamplerDesc {
    fn default() -> Self {
        Self {
            filter: Filter::Linear,
            address_u: TextureAddressMode::Repeat,
            address_v: TextureAddressMode::Repeat,
            address_w: TextureAddressMode::Repeat,
            mip_lod_bias: 0.0,
            max_anisotropy: 0,
            compare_op: CompareOp::Always,
            min_lod: 0.0,
            max_lod: f32::MAX,
        }
    }
}

pub struct TextureSubresourceLayout {
    pub offset_in_bytes: u64,
    pub row_pitch_in_bytes: u64,
    pub size_in_bytes: u64,
}

pub const MAX_RENDER_PASS_RENDER_TARGET_COUNT: usize = 8;

pub trait Device: Send + Sync {
    fn begin_frame(&self);
    fn end_frame(&self);

    fn create_buffer(&self, info: &BufferDesc, name: &str) -> Result<Buffer, DeviceError>;
    fn create_texture(&self, info: &TextureDesc, name: &str) -> Result<Texture, DeviceError>;
    fn create_shader_resource_view(
        &self,
        desc: &ShaderResourceViewDesc,
    ) -> Result<ShaderResourceView, DeviceError>;
    fn create_render_target_view(
        &self,
        desc: &RenderTargetViewDesc,
    ) -> Result<RenderTargetView, DeviceError>;
    fn create_swapchain(
        &self,
        info: &SwapChainDesc,
        old_swapchain: Option<SwapChain>,
    ) -> Result<SwapChain, DeviceError>;
    fn create_shader_module(&self, bytecode: &[u8]) -> Result<ShaderModule, DeviceError>;

    /// Create a transient command list
    /// Command lists are only one-frame objects and must not be recycled
    /// as there are handled by the backend
    fn create_command_list(&self, queue_type: QueueType) -> Result<CommandList, DeviceError>;
    fn create_sampler(&self, desc: &SamplerDesc) -> Result<Sampler, DeviceError>;

    // Buffer functions
    fn get_buffer_mapped_ptr(&self, buffer: &Buffer) -> Option<*mut u8>;

    // Texture functions
    fn get_texture_subresource_layout(
        &self,
        texture: &Texture,
        subresource_index: u32,
    ) -> TextureSubresourceLayout;

    // Swapchain functions
    fn get_swapchain_backbuffer_count(&self, swapchain: &SwapChain) -> usize;
    fn get_swapchain_backbuffer_index(&self, swapchain: &SwapChain) -> u32;
    fn get_swapchain_backbuffer(
        &self,
        swapchain: &SwapChain,
        index: u32,
    ) -> Result<Arc<Texture>, DeviceError>;
    fn present(&self, swapchain: &SwapChain);

    // Transfer functions
    fn cmd_copy_buffer_regions(
        &self,
        cmd_list: &mut CommandList,
        src_buffer: &Buffer,
        dst_buffer: &Buffer,
        regions: &[BufferCopyRegion],
    );
    fn cmd_copy_buffer_to_texture_regions(
        &self,
        cmd_list: &mut CommandList,
        src_buffer: &Buffer,
        dst_texture: &Texture,
        regions: &[BufferToTextureCopyRegion],
    );

    // Debug functions
    fn cmd_debug_begin_event(&self, cmd_list: &mut CommandList, name: &str, color: Color4f32);
    fn cmd_debug_end_event(&self, cmd_list: &mut CommandList);

    // Render passes functions
    fn cmd_begin_render_pass(&self, cmd_list: &mut CommandList, desc: &RenderPassDesc);
    fn cmd_end_render_pass(&self, cmd_list: &mut CommandList);
    fn cmd_resource_barrier(&self, cmd_list: &mut CommandList, barriers: &[ResourceBarrier]);
    fn cmd_set_viewports(&self, cmd_list: &mut CommandList, viewports: &[Viewport]);
    fn cmd_set_scissors(&self, cmd_list: &mut CommandList, scissors: &[RectI32]);

    // Pipeline functions
    fn cmd_set_shader_stages(&self, cmd_list: &mut CommandList, stages: &[PipelineShaderStage]);
    fn cmd_set_input_assembly_state(
        &self,
        cmd_list: &mut CommandList,
        state: &PipelineInputAssemblyState,
    );
    fn cmd_set_blend_state(&self, cmd_list: &mut CommandList, state: &PipelineBlendState);
    fn cmd_bind_index_buffer(
        &self,
        cmd_list: &mut CommandList,
        index_buffer: &Buffer,
        format: IndexBufferFormat,
    );
    fn cmd_push_constants(&self, cmd_list: &mut CommandList, offset_in_bytes: u32, data: &[u8]);
    fn cmd_draw(
        &self,
        cmd_list: &mut CommandList,
        vertex_count_per_instance: u32,
        instance_count: u32,
        first_vertex: u32,
        first_instance: u32,
    );
    fn cmd_draw_indexed(
        &self,
        cmd_list: &mut CommandList,
        index_count_per_instance: u32,
        instance_count: u32,
        first_index: u32,
        first_instance: u32,
    );

    /// Submit work to a specific queue to the GPU, optionally waiting or signaling fences
    fn submit(
        &self,
        queue_type: QueueType,
        command_lists: &[&CommandList],
        wait_fences: &[&Fence],
        signal_fences: &[&Fence],
    );

    // Block the current thread until all GPU queues are flushed
    fn wait_idle(&self);
}

// Resources
#[derive(Copy, Clone, PartialEq, Eq)]
pub enum MemoryLocation {
    CpuToGpu,
    GpuOnly,
}

#[bitflags]
#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq)]
#[repr(u32)]
pub enum BufferUsageFlagBits {
    UnorderedAccess = 1 << 0,
    IndexBuffer = 1 << 1,
}
pub type BufferUsageFlags = BitFlags<BufferUsageFlagBits>;

// Data describing a buffer, persistent and always accessible from the buffer
#[derive(Copy, Clone)]
pub struct BufferDesc {
    pub size_bytes: u64,
    pub usage: BufferUsageFlags,
    pub memory_location: MemoryLocation,
    pub default_resource_state: ResourceState,
}

pub struct Buffer {
    pub info: BufferDesc,
    pub backend_data: Box<dyn Any + Send + Sync>,
}

impl Buffer {
    pub fn new(info: &BufferDesc, backend_data: Box<dyn Any + Send + Sync>) -> Self {
        Self {
            info: info.clone(),
            backend_data,
        }
    }
}

#[bitflags]
#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Debug)]
#[repr(u32)]
pub enum TextureUsageFlagBits {
    UnorderedAccess = 1 << 0,
    RenderTarget = 1 << 1,
}
pub type TextureUsageFlags = BitFlags<TextureUsageFlagBits>;

#[derive(Copy, Clone)]
pub struct TextureDesc {
    pub width: u32,
    pub height: u32,
    pub depth: u32,
    pub mip_levels: u32,
    pub format: PixelFormat,
    pub sample_desc: SampleDesc,
    pub usage_flags: TextureUsageFlags,
    pub memory_location: MemoryLocation,
}

#[derive(Copy, Clone, Eq, PartialEq)]
pub struct TextureSubresourceRange {
    pub base_mip_level: u32,
    pub level_count: u32,
}

pub struct Texture {
    pub desc: TextureDesc,
    pub backend_data: Box<dyn Any + Send + Sync>,
}

impl Texture {
    pub fn new(desc: TextureDesc, backend_data: Box<dyn Any + Send + Sync>) -> Self {
        Self { desc, backend_data }
    }
}

pub struct Sampler {
    pub desc: SamplerDesc,
    pub backend_data: Box<dyn ShaderVisibleResource>,
}

impl Sampler {
    pub fn new(desc: SamplerDesc, backend_data: Box<dyn ShaderVisibleResource>) -> Self {
        Self { desc, backend_data }
    }

    pub fn get_descriptor_index(&self) -> u32 {
        self.backend_data.get_descriptor_index()
    }
}

pub trait ShaderVisibleResource: Any + Send {
    fn get_descriptor_index(&self) -> u32;
}

pub struct ShaderResourceView {
    pub desc: ShaderResourceViewDesc,
    pub backend_data: Box<dyn ShaderVisibleResource>,
}

impl ShaderResourceView {
    pub fn new(desc: ShaderResourceViewDesc, backend_data: Box<dyn ShaderVisibleResource>) -> Self {
        Self { desc, backend_data }
    }

    pub fn get_descriptor_index(&self) -> u32 {
        self.backend_data.get_descriptor_index()
    }
}

pub struct RenderTargetView {
    pub desc: RenderTargetViewDesc,
    pub backend_data: Box<dyn Any + Send>,
}

impl RenderTargetView {
    pub fn new(desc: RenderTargetViewDesc, backend_data: Box<dyn Any + Send>) -> Self {
        Self { desc, backend_data }
    }
}

pub struct CommandList {
    pub backend_data: Box<dyn Any + Send>,
}

impl CommandList {
    pub fn new(backend_data: Box<dyn Any + Send>) -> Self {
        Self { backend_data }
    }
}

pub struct ShaderModule {
    pub backend_data: Box<dyn Any + Send + Sync>,
}

impl ShaderModule {
    pub fn new(backend_data: Box<dyn Any + Send + Sync>) -> Self {
        Self { backend_data }
    }
}

#[derive(Copy, Clone, Debug)]
pub struct SwapChainDesc {
    pub width: u32,
    pub height: u32,
    pub format: PixelFormat,
    pub sample_desc: SampleDesc,
    pub usage_flags: TextureUsageFlags,
    pub window_handle: RawWindowHandle,
}

#[derive(Debug)]
pub struct SwapChain {
    pub info: SwapChainDesc,
    pub backend_data: Box<dyn Any + Send + Sync>,
}

impl SwapChain {
    pub fn new(info: SwapChainDesc, backend_data: Box<dyn Any + Send + Sync>) -> Self {
        Self { info, backend_data }
    }
}

pub struct Fence {}
