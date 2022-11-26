use crate::frame_manager::FrameManager;
use crate::resource_manager::Entry;
use crate::utils::SendableIUnknown;
use std::sync::Arc;
use windows::Win32::Graphics::Direct3D12::*;
use ze_d3dmemoryallocator::Allocation;

pub(crate) struct D3D12Buffer {
    pub frame_manager: Arc<FrameManager>,
    pub resource: SendableIUnknown<ID3D12Resource>,
    pub allocation: Option<Allocation>,
    pub mapped_ptr: Option<*mut u8>,
    pub gpu_virtual_address: u64,
}

unsafe impl Send for D3D12Buffer {}
unsafe impl Sync for D3D12Buffer {}

impl D3D12Buffer {
    pub fn new(
        frame_manager: Arc<FrameManager>,
        resource: SendableIUnknown<ID3D12Resource>,
        allocation: Option<Allocation>,
        mapped_ptr: Option<*mut u8>,
        gpu_virtual_address: u64,
    ) -> Self {
        Self {
            frame_manager,
            resource,
            allocation,
            mapped_ptr,
            gpu_virtual_address,
        }
    }
}

impl Drop for D3D12Buffer {
    fn drop(&mut self) {
        self.frame_manager
            .current_frame()
            .resource_queue()
            .push(Entry::Resource(self.resource.clone()));

        if let Some(allocation) = self.allocation.take() {
            self.frame_manager
                .current_frame()
                .resource_queue()
                .push(Entry::Allocation(allocation));
        }
    }
}
