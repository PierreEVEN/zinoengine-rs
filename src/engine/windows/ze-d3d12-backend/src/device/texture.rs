use crate::frame_manager::FrameManager;
use crate::resource_manager::Entry;
use crate::utils::SendableIUnknown;
use std::sync::Arc;
use windows::Win32::Graphics::Direct3D12::ID3D12Resource;
use ze_d3dmemoryallocator::Allocation;

pub(crate) struct D3D12Texture {
    pub frame_manager: Arc<FrameManager>,
    pub texture: SendableIUnknown<ID3D12Resource>,
    pub allocation: Option<Allocation>,
}

impl D3D12Texture {
    pub fn new(
        frame_manager: Arc<FrameManager>,
        texture: SendableIUnknown<ID3D12Resource>,
        allocation: Option<Allocation>,
    ) -> Self {
        Self {
            frame_manager,
            texture,
            allocation,
        }
    }
}

impl Drop for D3D12Texture {
    fn drop(&mut self) {
        self.frame_manager
            .current_frame()
            .resource_queue()
            .push(Entry::Resource(self.texture.clone()));

        if let Some(allocation) = self.allocation.take() {
            self.frame_manager
                .current_frame()
                .resource_queue()
                .push(Entry::Allocation(allocation));
        }
    }
}
