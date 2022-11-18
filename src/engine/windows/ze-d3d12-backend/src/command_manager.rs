use crate::device::cmd_list::D3D12CommandList;
use crate::device::D3D12Device;
use crate::utils;
use crate::utils::SendableIUnknown;
use std::cell::RefCell;
use std::collections::{HashMap, VecDeque};
use std::ops::Deref;
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread;
use thread_local::ThreadLocal;
use tinyvec::TinyVec;
use windows::core::Interface;
use windows::Win32::Graphics::Direct3D12::*;
use ze_core::pool::{Handle, Pool};
use ze_gfx::backend;
use ze_gfx::backend::{Fence, QueueType};

pub struct CommandList {
    pub command_list: SendableIUnknown<ID3D12GraphicsCommandList6>,
}

impl CommandList {
    pub fn new(command_list: SendableIUnknown<ID3D12GraphicsCommandList6>) -> Self {
        Self { command_list }
    }
}

struct CommandAllocator {
    allocator: SendableIUnknown<ID3D12CommandAllocator>,
    list_pool: Pool<CommandList, 32>,
    free_lists: VecDeque<Handle<CommandList>>,
    allocated_lists: VecDeque<Handle<CommandList>>,
}

impl CommandAllocator {
    pub fn new(allocator: SendableIUnknown<ID3D12CommandAllocator>) -> Self {
        Self {
            allocator,
            list_pool: Default::default(),
            free_lists: Default::default(),
            allocated_lists: Default::default(),
        }
    }

    pub fn allocate(
        &mut self,
        device: &D3D12Device,
        list_type: D3D12_COMMAND_LIST_TYPE,
    ) -> (
        Handle<CommandList>,
        SendableIUnknown<ID3D12GraphicsCommandList6>,
    ) {
        if let Some(list) = self.free_lists.pop_front() {
            let command_list = &mut self.list_pool[&list];
            unsafe {
                command_list
                    .command_list
                    .Reset(self.allocator.deref(), None)
                    .unwrap_unchecked();
            }
            self.allocated_lists.push_back(list);
            (list, command_list.command_list.clone())
        } else {
            let list: windows::core::Result<ID3D12GraphicsCommandList> = unsafe {
                device.device().CreateCommandList(
                    0,
                    list_type,
                    &self.allocator.deref().clone(),
                    None,
                )
            };

            let list = list.unwrap().cast::<ID3D12GraphicsCommandList6>().unwrap();

            let handle = self.list_pool.insert(CommandList::new(list.clone().into()));
            self.allocated_lists.push_back(handle);
            (handle, list.into())
        }
    }

    pub fn reset(&mut self) {
        unsafe {
            self.allocator.Reset().unwrap_unchecked();
        }

        self.free_lists.append(&mut self.allocated_lists);
    }
}

struct SyncRefCell<T>(RefCell<T>);
unsafe impl<T> Sync for SyncRefCell<T> {}

struct CommandQueue {
    ty: QueueType,
    queue: SendableIUnknown<ID3D12CommandQueue>,
    allocators: ThreadLocal<SyncRefCell<CommandAllocator>>,
    work_fence: SendableIUnknown<ID3D12Fence>,
    fence_counter: AtomicU64,
}

impl CommandQueue {
    fn new(ty: QueueType, queue: &ID3D12CommandQueue, fence: ID3D12Fence) -> Self {
        Self {
            ty,
            queue: queue.clone().into(),
            allocators: Default::default(),
            work_fence: fence.into(),
            fence_counter: AtomicU64::new(0),
        }
    }

    fn wait_for_work(&self) {
        let prev_counter = self.fence_counter.fetch_add(1, Ordering::SeqCst);

        unsafe {
            self.queue
                .Signal(self.work_fence.deref(), prev_counter + 1)
                .unwrap_unchecked();

            while self.work_fence.GetCompletedValue() < (prev_counter + 1) {
                thread::yield_now();
            }
        }
    }

    fn reset(&self) {
        for allocator in self.allocators.iter() {
            allocator.0.borrow_mut().reset();
        }
    }

    fn submit(
        &self,
        command_lists: &[&backend::CommandList],
        wait_fences: &[&Fence],
        signal_fences: &[&Fence],
    ) {
        for _ in wait_fences {
            todo!()
            //let current_value = fence.
            //self.queue.Wait();
        }

        let command_lists = {
            let mut lists: TinyVec<[Option<ID3D12CommandList>; 8]> = TinyVec::new();

            for command_list in command_lists {
                let command_list = unsafe {
                    command_list
                        .backend_data
                        .downcast_ref::<D3D12CommandList>()
                        .unwrap_unchecked()
                };

                unsafe {
                    command_list.cmd_list().Close().unwrap_unchecked();
                    lists.push(Some(
                        command_list
                            .cmd_list()
                            .cast::<ID3D12CommandList>()
                            .unwrap_unchecked(),
                    ));
                }
            }

            lists
        };

        unsafe {
            self.queue.ExecuteCommandLists(&command_lists);
        }

        for _ in signal_fences {
            todo!()
            //let current_value = fence.
            //self.queue.Wait();
        }
    }

    fn get_or_create_allocator(
        &self,
        device: &D3D12Device,
        comannd_list_type: D3D12_COMMAND_LIST_TYPE,
    ) -> &SyncRefCell<CommandAllocator> {
        self.allocators.get_or(|| {
            let allocator: ID3D12CommandAllocator =
                unsafe { device.device().CreateCommandAllocator(comannd_list_type) }.unwrap();

            utils::set_resource_name(
                &allocator.clone().into(),
                &format!("Command Allocator (Thread: {:?})", thread::current().id()),
            );

            SyncRefCell(RefCell::new(CommandAllocator::new(allocator.into())))
        })
    }
}

/// Manage command queues, allocators and lists
/// When a command list is allocated, it'll be recycled on the next frame
/// There is a set of alloctors per thread and one allocator per command list type
pub(crate) struct CommandManager {
    queues: HashMap<QueueType, CommandQueue>,
}

impl CommandManager {
    pub fn new(
        device: &ID3D12Device2,
        graphics_queue: &ID3D12CommandQueue,
        compute_queue: &ID3D12CommandQueue,
        transfer_queue: &ID3D12CommandQueue,
    ) -> Self {
        let graphics_queue_fence: windows::core::Result<ID3D12Fence> =
            unsafe { device.CreateFence(0, D3D12_FENCE_FLAG_NONE) };

        let compute_queue_fence: windows::core::Result<ID3D12Fence> =
            unsafe { device.CreateFence(0, D3D12_FENCE_FLAG_NONE) };

        let transfer_queue_fence: windows::core::Result<ID3D12Fence> =
            unsafe { device.CreateFence(0, D3D12_FENCE_FLAG_NONE) };

        let mut queues = HashMap::new();
        queues.insert(
            QueueType::Graphics,
            CommandQueue::new(
                QueueType::Graphics,
                graphics_queue,
                graphics_queue_fence.unwrap(),
            ),
        );
        queues.insert(
            QueueType::Compute,
            CommandQueue::new(
                QueueType::Compute,
                compute_queue,
                compute_queue_fence.unwrap(),
            ),
        );
        queues.insert(
            QueueType::Transfer,
            CommandQueue::new(
                QueueType::Transfer,
                transfer_queue,
                transfer_queue_fence.unwrap(),
            ),
        );

        Self { queues }
    }

    pub fn new_frame(&self) {
        for queue in self.queues.values() {
            queue.wait_for_work();
            queue.reset();
        }
    }

    pub fn create_command_list(
        &self,
        device: &D3D12Device,
        queue_type: QueueType,
    ) -> (
        Handle<CommandList>,
        SendableIUnknown<ID3D12GraphicsCommandList6>,
    ) {
        if let Some(queue) = self.queues.get(&queue_type) {
            debug_assert_eq!(queue.ty, queue_type);

            let command_list_type = match queue_type {
                QueueType::Graphics => D3D12_COMMAND_LIST_TYPE_DIRECT,
                QueueType::Compute => D3D12_COMMAND_LIST_TYPE_COMPUTE,
                QueueType::Transfer => D3D12_COMMAND_LIST_TYPE_COPY,
            };

            queue
                .get_or_create_allocator(device, command_list_type)
                .0
                .borrow_mut()
                .allocate(device, command_list_type)
        } else {
            panic!("Queue not found");
        }
    }

    pub fn wait_for_work(&self) {
        for queue in self.queues.values() {
            queue.wait_for_work();
        }
    }

    pub fn submit(
        &self,
        queue_type: QueueType,
        command_lists: &[&backend::CommandList],
        wait_fences: &[&Fence],
        signal_fences: &[&Fence],
    ) {
        if let Some(queue) = self.queues.get(&queue_type) {
            queue.submit(command_lists, wait_fences, signal_fences);
        } else {
            panic!("Queue not found");
        }
    }
}
