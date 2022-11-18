use crate::command_manager::CommandManager;
use crate::device::D3D12Device;
use std::sync::atomic::{AtomicUsize, Ordering};
use windows::Win32::Graphics::Direct3D12::{ID3D12CommandQueue, ID3D12Device2};

use crate::resource_manager::DeferredResourceQueue;

pub(crate) struct Frame {
    resource_manager: DeferredResourceQueue,
    command_manager: CommandManager,
}

impl Frame {
    pub fn new(
        device: &ID3D12Device2,
        graphics_queue: &ID3D12CommandQueue,
        compute_queue: &ID3D12CommandQueue,
        transfer_queue: &ID3D12CommandQueue,
    ) -> Self {
        Self {
            resource_manager: DeferredResourceQueue::default(),
            command_manager: CommandManager::new(
                device,
                graphics_queue,
                compute_queue,
                transfer_queue,
            ),
        }
    }

    /// Wait all works issued by this frame has been finished and reset all commands
    pub fn wait_for_work(&self) {
        self.command_manager.wait_for_work();
    }

    pub fn resource_manager(&self) -> &DeferredResourceQueue {
        &self.resource_manager
    }

    pub fn command_manager(&self) -> &CommandManager {
        &self.command_manager
    }
}

/// Manage multiple frames that may be processed concurrently without concerns
pub(crate) struct FrameManager {
    frames: Vec<Frame>,
    frame_count: usize,
    current_frame: AtomicUsize,
}

impl FrameManager {
    pub fn new(
        frame_count: usize,
        device: &ID3D12Device2,
        graphics_queue: &ID3D12CommandQueue,
        compute_queue: &ID3D12CommandQueue,
        transfer_queue: &ID3D12CommandQueue,
    ) -> Self {
        let mut frames = vec![];
        for _ in 0..frame_count {
            frames.push(Frame::new(
                device,
                graphics_queue,
                compute_queue,
                transfer_queue,
            ));
        }

        Self {
            frames,
            frame_count,
            current_frame: AtomicUsize::new(0),
        }
    }

    pub fn begin_frame(&self, _: &D3D12Device) {
        self.current_frame
            .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |old_frame| {
                Some((old_frame + 1) % self.frame_count)
            })
            .unwrap();

        self.current_frame().command_manager().new_frame();

        self.current_frame().resource_manager().flush();
    }

    pub fn wait_for_work(&self) {
        for frame in &self.frames {
            frame.wait_for_work();
        }
    }

    pub fn current_frame(&self) -> &Frame {
        &self.frames[self.current_frame.load(Ordering::SeqCst)]
    }

    pub fn frame_count(&self) -> usize {
        self.frame_count
    }
}
