use crate::device::SendableAllocator;
use crate::utils::SendableIUnknown;
use gpu_allocator::d3d12::Allocation;
use parking_lot::Mutex;
use std::sync::Arc;
use windows::Win32::Graphics::Direct3D12::ID3D12Resource;
use ze_core::ze_info;

pub struct DeferredDestructionResourceEntry {
    pub resource: SendableIUnknown<ID3D12Resource>,
    pub allocation: Option<Allocation>,
}

/// This object manage resource lifetimes in a elegant way
/// Providing a way to defer destruction and managing multiple frames
pub struct ResourceManager {
    allocator: Arc<Mutex<SendableAllocator>>,
    queue: Mutex<Vec<DeferredDestructionResourceEntry>>,
}

impl ResourceManager {
    pub fn new(allocator: Arc<Mutex<SendableAllocator>>) -> Self {
        Self {
            allocator,
            queue: Default::default(),
        }
    }

    /// Enqueue destruction of a resource in a closure
    pub fn enqueue_resource_destruction(
        &self,
        resource: SendableIUnknown<ID3D12Resource>,
        allocation: Option<Allocation>,
    ) {
        self.queue.lock().push(DeferredDestructionResourceEntry {
            resource,
            allocation,
        })
    }

    pub fn destroy_resources(&self) {
        let mut allocator = self.allocator.lock();
        let mut queue = self.queue.lock();

        if !queue.is_empty() {
            ze_info!(
                "(Deferred destruction) Destroying {} resources",
                queue.len()
            );
        }

        for entry in queue.drain(..) {
            if let Some(allocation) = entry.allocation {
                allocator.0.free(allocation).unwrap();
            }
        }
    }
}

impl Drop for ResourceManager {
    fn drop(&mut self) {
        self.destroy_resources();
    }
}
