use crate::utils::SendableIUnknown;
use parking_lot::Mutex;
use windows::Win32::Graphics::Direct3D12::ID3D12Resource;
use ze_d3dmemoryallocator::Allocation;

pub enum Entry {
    Resource(SendableIUnknown<ID3D12Resource>),
    Allocation(Allocation),
}

/// This object manage resource lifetimes in a elegant way
/// Providing a way to defer destruction and managing multiple frames
#[derive(Default)]
pub(crate) struct DeferredResourceQueue {
    queue: Mutex<Vec<Entry>>,
}

impl DeferredResourceQueue {
    pub fn push(&self, entry: Entry) {
        self.queue.lock().push(entry)
    }

    pub fn flush(&self) {
        self.queue.lock().clear();
    }
}

impl Drop for DeferredResourceQueue {
    fn drop(&mut self) {
        self.flush();
        assert!(self.queue.lock().is_empty());
    }
}
