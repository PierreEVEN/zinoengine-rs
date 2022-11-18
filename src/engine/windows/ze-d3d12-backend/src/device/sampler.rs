use crate::descriptor_manager::DescriptorManager;
use std::sync::Arc;
use windows::Win32::Graphics::Direct3D12::D3D12_CPU_DESCRIPTOR_HANDLE;
use ze_gfx::backend::ShaderVisibleResource;

pub(crate) struct D3D12Sampler {
    pub descriptor_manager: Arc<DescriptorManager>,
    pub handle: (D3D12_CPU_DESCRIPTOR_HANDLE, u32),
}

impl Drop for D3D12Sampler {
    fn drop(&mut self) {
        self.descriptor_manager
            .free_sampler_descriptor_handle(self.handle);
    }
}

impl ShaderVisibleResource for D3D12Sampler {
    fn descriptor_index(&self) -> u32 {
        self.handle.1
    }
}
