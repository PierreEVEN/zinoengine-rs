use crate::backend::*;
use std::sync::Arc;
use ze_core::color::Color4f32;
use ze_core::maths::RectI32;

#[derive(Default)]
pub struct NullBackend;

impl Backend for NullBackend {
    fn create_device(&self) -> Result<Arc<dyn Device>, BackendError> {
        Ok(Arc::new(NullDevice::default()))
    }

    fn name(&self) -> &str {
        "Null"
    }
}

#[derive(Default)]
struct NullDevice;

impl Device for NullDevice {
    fn begin_frame(&self) {}

    fn end_frame(&self) {}

    fn create_buffer(
        &self,
        _: &BufferDesc,
        _: Option<&MemoryPool>,
        _: &str,
    ) -> Result<Buffer, DeviceError> {
        Err(DeviceError::Unknown)
    }

    fn create_texture(
        &self,
        _: &TextureDesc,
        _: Option<&MemoryPool>,
        _: &str,
    ) -> Result<Texture, DeviceError> {
        Err(DeviceError::Unknown)
    }

    fn create_shader_resource_view(
        &self,
        _: &ShaderResourceViewDesc,
    ) -> Result<ShaderResourceView, DeviceError> {
        Err(DeviceError::Unknown)
    }

    fn create_render_target_view(
        &self,
        _: &RenderTargetViewDesc,
    ) -> Result<RenderTargetView, DeviceError> {
        Err(DeviceError::Unknown)
    }

    fn create_depth_stencil_view(
        &self,
        _: &DepthStencilViewDesc,
    ) -> Result<DepthStencilView, DeviceError> {
        Err(DeviceError::Unknown)
    }

    fn create_swapchain(
        &self,
        _: &SwapChainDesc,
        _: Option<SwapChain>,
    ) -> Result<SwapChain, DeviceError> {
        Err(DeviceError::Unknown)
    }

    fn create_shader_module(&self, _: &[u8]) -> Result<ShaderModule, DeviceError> {
        Err(DeviceError::Unknown)
    }

    fn create_command_list(&self, _: QueueType) -> Result<CommandList, DeviceError> {
        Err(DeviceError::Unknown)
    }

    fn create_sampler(&self, _: &SamplerDesc) -> Result<Sampler, DeviceError> {
        Err(DeviceError::Unknown)
    }

    fn buffer_mapped_ptr(&self, _: &Buffer) -> Option<*mut u8> {
        None
    }

    fn texture_subresource_layout(&self, _: &Texture, _: u32) -> TextureSubresourceLayout {
        TextureSubresourceLayout {
            offset_in_bytes: 0,
            row_pitch_in_bytes: 0,
            size_in_bytes: 0,
        }
    }

    fn swapchain_backbuffer_count(&self, _: &SwapChain) -> usize {
        0
    }

    fn swapchain_backbuffer_index(&self, _: &SwapChain) -> u32 {
        0
    }

    fn swapchain_backbuffer(&self, _: &SwapChain, _: u32) -> Result<Arc<Texture>, DeviceError> {
        Err(DeviceError::Unknown)
    }

    fn present(&self, _: &SwapChain) {}

    fn transient_memory_pool(&self) -> &MemoryPool {
        unimplemented!()
    }

    fn cmd_copy_buffer_regions(
        &self,
        _: &mut CommandList,
        _: &Buffer,
        _: &Buffer,
        _: &[BufferCopyRegion],
    ) {
    }

    fn cmd_copy_buffer_to_texture_regions(
        &self,
        _: &mut CommandList,
        _: &Buffer,
        _: &Texture,
        _: &[BufferToTextureCopyRegion],
    ) {
    }

    fn cmd_debug_begin_event(&self, _: &mut CommandList, _: &str, _: Color4f32) {}

    fn cmd_debug_end_event(&self, _: &mut CommandList) {}

    fn cmd_begin_render_pass(&self, _: &mut CommandList, _: &RenderPassDesc) {}

    fn cmd_end_render_pass(&self, _: &mut CommandList) {}

    fn cmd_resource_barrier(&self, _: &mut CommandList, _: &[ResourceBarrier]) {}

    fn cmd_set_viewports(&self, _: &mut CommandList, _: &[Viewport]) {}

    fn cmd_set_scissors(&self, _: &mut CommandList, _: &[RectI32]) {}

    fn cmd_set_shader_stages(&self, _: &mut CommandList, _: &[PipelineShaderStage]) {}

    fn cmd_set_input_assembly_state(&self, _: &mut CommandList, _: &PipelineInputAssemblyState) {}

    fn cmd_set_blend_state(&self, _: &mut CommandList, _: &PipelineBlendState) {}

    fn cmd_set_depth_stencil_state(&self, _: &mut CommandList, _: &PipelineDepthStencilState) {}

    fn cmd_bind_index_buffer(&self, _: &mut CommandList, _: &Buffer, _: IndexBufferFormat) {}

    fn cmd_push_constants(&self, _: &mut CommandList, _: u32, _: &[u8]) {}

    fn cmd_draw(&self, _: &mut CommandList, _: u32, _: u32, _: u32, _: u32) {}

    fn cmd_draw_indexed(&self, _: &mut CommandList, _: u32, _: u32, _: u32, _: u32) {}

    fn cmd_dispatch_mesh(&self, _: &mut CommandList, _: u32, _: u32, _: u32) {}

    fn submit(&self, _: QueueType, _: &[&CommandList], _: &[&Fence], _: &[&Fence]) {}

    fn wait_idle(&self) {}
}
