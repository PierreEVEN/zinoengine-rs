use ze_gfx::backend::Device;

struct SyncDevice(metal::Device);

unsafe impl Send for SyncDevice {}
unsafe impl Sync for SyncDevice {}

pub struct MetalDevice {
    device: SyncDevice,
}

impl MetalDevice {
    pub fn new(device: metal::Device) -> Self {
        Self {
            device: SyncDevice(device),
        }
    }
}

impl Device for MetalDevice {
    fn begin_frame(&self) {
        todo!()
    }

    fn end_frame(&self) {
        todo!()
    }

    fn create_buffer(
        &self,
        info: &ze_gfx::backend::BufferDesc,
        name: &str,
    ) -> Result<ze_gfx::backend::Buffer, ze_gfx::backend::DeviceError> {
        todo!()
    }

    fn create_texture(
        &self,
        info: &ze_gfx::backend::TextureDesc,
        name: &str,
    ) -> Result<ze_gfx::backend::Texture, ze_gfx::backend::DeviceError> {
        todo!()
    }

    fn create_shader_resource_view(
        &self,
        desc: &ze_gfx::backend::ShaderResourceViewDesc,
    ) -> Result<ze_gfx::backend::ShaderResourceView, ze_gfx::backend::DeviceError> {
        todo!()
    }

    fn create_render_target_view(
        &self,
        desc: &ze_gfx::backend::RenderTargetViewDesc,
    ) -> Result<ze_gfx::backend::RenderTargetView, ze_gfx::backend::DeviceError> {
        todo!()
    }

    fn create_swapchain(
        &self,
        info: &ze_gfx::backend::SwapChainDesc,
        old_swapchain: Option<ze_gfx::backend::SwapChain>,
    ) -> Result<ze_gfx::backend::SwapChain, ze_gfx::backend::DeviceError> {
        todo!()
    }

    fn create_shader_module(
        &self,
        bytecode: &[u8],
    ) -> Result<ze_gfx::backend::ShaderModule, ze_gfx::backend::DeviceError> {
        todo!()
    }

    fn create_command_list(
        &self,
        queue_type: ze_gfx::backend::QueueType,
    ) -> Result<ze_gfx::backend::CommandList, ze_gfx::backend::DeviceError> {
        todo!()
    }

    fn create_sampler(
        &self,
        desc: &ze_gfx::backend::SamplerDesc,
    ) -> Result<ze_gfx::backend::Sampler, ze_gfx::backend::DeviceError> {
        todo!()
    }

    fn buffer_mapped_ptr(&self, buffer: &ze_gfx::backend::Buffer) -> Option<*mut u8> {
        todo!()
    }

    fn texture_subresource_layout(
        &self,
        texture: &ze_gfx::backend::Texture,
        subresource_index: u32,
    ) -> ze_gfx::backend::TextureSubresourceLayout {
        todo!()
    }

    fn swapchain_backbuffer_count(&self, swapchain: &ze_gfx::backend::SwapChain) -> usize {
        todo!()
    }

    fn swapchain_backbuffer_index(&self, swapchain: &ze_gfx::backend::SwapChain) -> u32 {
        todo!()
    }

    fn swapchain_backbuffer(
        &self,
        swapchain: &ze_gfx::backend::SwapChain,
        index: u32,
    ) -> Result<std::sync::Arc<ze_gfx::backend::Texture>, ze_gfx::backend::DeviceError> {
        todo!()
    }

    fn present(&self, swapchain: &ze_gfx::backend::SwapChain) {
        todo!()
    }

    fn cmd_copy_buffer_regions(
        &self,
        cmd_list: &mut ze_gfx::backend::CommandList,
        src_buffer: &ze_gfx::backend::Buffer,
        dst_buffer: &ze_gfx::backend::Buffer,
        regions: &[ze_gfx::backend::BufferCopyRegion],
    ) {
        todo!()
    }

    fn cmd_copy_buffer_to_texture_regions(
        &self,
        cmd_list: &mut ze_gfx::backend::CommandList,
        src_buffer: &ze_gfx::backend::Buffer,
        dst_texture: &ze_gfx::backend::Texture,
        regions: &[ze_gfx::backend::BufferToTextureCopyRegion],
    ) {
        todo!()
    }

    fn cmd_debug_begin_event(
        &self,
        cmd_list: &mut ze_gfx::backend::CommandList,
        name: &str,
        color: ze_core::color::Color4f32,
    ) {
        todo!()
    }

    fn cmd_debug_end_event(&self, cmd_list: &mut ze_gfx::backend::CommandList) {
        todo!()
    }

    fn cmd_begin_render_pass(
        &self,
        cmd_list: &mut ze_gfx::backend::CommandList,
        desc: &ze_gfx::backend::RenderPassDesc,
    ) {
        todo!()
    }

    fn cmd_end_render_pass(&self, cmd_list: &mut ze_gfx::backend::CommandList) {
        todo!()
    }

    fn cmd_resource_barrier(
        &self,
        cmd_list: &mut ze_gfx::backend::CommandList,
        barriers: &[ze_gfx::backend::ResourceBarrier],
    ) {
        todo!()
    }

    fn cmd_set_viewports(
        &self,
        cmd_list: &mut ze_gfx::backend::CommandList,
        viewports: &[ze_gfx::backend::Viewport],
    ) {
        todo!()
    }

    fn cmd_set_scissors(
        &self,
        cmd_list: &mut ze_gfx::backend::CommandList,
        scissors: &[ze_core::maths::RectI32],
    ) {
        todo!()
    }

    fn cmd_set_shader_stages(
        &self,
        cmd_list: &mut ze_gfx::backend::CommandList,
        stages: &[ze_gfx::backend::PipelineShaderStage],
    ) {
        todo!()
    }

    fn cmd_set_input_assembly_state(
        &self,
        cmd_list: &mut ze_gfx::backend::CommandList,
        state: &ze_gfx::backend::PipelineInputAssemblyState,
    ) {
        todo!()
    }

    fn cmd_set_blend_state(
        &self,
        cmd_list: &mut ze_gfx::backend::CommandList,
        state: &ze_gfx::backend::PipelineBlendState,
    ) {
        todo!()
    }

    fn cmd_bind_index_buffer(
        &self,
        cmd_list: &mut ze_gfx::backend::CommandList,
        index_buffer: &ze_gfx::backend::Buffer,
        format: ze_gfx::backend::IndexBufferFormat,
    ) {
        todo!()
    }

    fn cmd_push_constants(
        &self,
        cmd_list: &mut ze_gfx::backend::CommandList,
        offset_in_bytes: u32,
        data: &[u8],
    ) {
        todo!()
    }

    fn cmd_draw(
        &self,
        cmd_list: &mut ze_gfx::backend::CommandList,
        vertex_count_per_instance: u32,
        instance_count: u32,
        first_vertex: u32,
        first_instance: u32,
    ) {
        todo!()
    }

    fn cmd_draw_indexed(
        &self,
        cmd_list: &mut ze_gfx::backend::CommandList,
        index_count_per_instance: u32,
        instance_count: u32,
        first_index: u32,
        first_instance: u32,
    ) {
        todo!()
    }

    fn submit(
        &self,
        queue_type: ze_gfx::backend::QueueType,
        command_lists: &[&ze_gfx::backend::CommandList],
        wait_fences: &[&ze_gfx::backend::Fence],
        signal_fences: &[&ze_gfx::backend::Fence],
    ) {
        todo!()
    }

    fn wait_idle(&self) {
        todo!()
    }
}
