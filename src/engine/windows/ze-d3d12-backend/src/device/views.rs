use crate::descriptor_manager::DescriptorManager;
use std::sync::Arc;
use windows::Win32::Graphics::Direct3D12::D3D12_CPU_DESCRIPTOR_HANDLE;
use ze_gfx::backend::ShaderVisibleResource;

pub(crate) struct D3D12RenderTargetView {
    pub descriptor_manager: Arc<DescriptorManager>,
    pub handle: (D3D12_CPU_DESCRIPTOR_HANDLE, u32),
}

impl Drop for D3D12RenderTargetView {
    fn drop(&mut self) {
        self.descriptor_manager
            .free_rtv_descriptor_handle(self.handle);
    }
}

pub(crate) struct D3D12DepthStencilView {
    pub descriptor_manager: Arc<DescriptorManager>,
    pub handle: (D3D12_CPU_DESCRIPTOR_HANDLE, u32),
}

impl Drop for D3D12DepthStencilView {
    fn drop(&mut self) {
        self.descriptor_manager
            .free_dsv_descriptor_handle(self.handle);
    }
}

pub struct D3D12ShaderResourceView {
    pub descriptor_manager: Arc<DescriptorManager>,
    pub handle: (D3D12_CPU_DESCRIPTOR_HANDLE, u32),
}

impl Drop for D3D12ShaderResourceView {
    fn drop(&mut self) {
        self.descriptor_manager
            .free_cbv_srv_uav_descriptor_handle(self.handle);
    }
}

impl ShaderVisibleResource for D3D12ShaderResourceView {
    fn descriptor_index(&self) -> u32 {
        self.handle.1
    }
}
