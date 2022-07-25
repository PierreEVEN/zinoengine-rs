use crate::utils;
use crate::utils::SendableIUnknown;
use parking_lot::Mutex;
use std::collections::VecDeque;
use windows::Win32::Graphics::Direct3D12::{
    ID3D12DescriptorHeap, ID3D12Device, D3D12_CPU_DESCRIPTOR_HANDLE, D3D12_DESCRIPTOR_HEAP_DESC,
    D3D12_DESCRIPTOR_HEAP_FLAG_NONE, D3D12_DESCRIPTOR_HEAP_FLAG_SHADER_VISIBLE,
    D3D12_DESCRIPTOR_HEAP_TYPE_CBV_SRV_UAV, D3D12_DESCRIPTOR_HEAP_TYPE_DSV,
    D3D12_DESCRIPTOR_HEAP_TYPE_RTV, D3D12_DESCRIPTOR_HEAP_TYPE_SAMPLER,
};

const MAX_CBV_SRV_UAV_DESCRIPTOR_COUNT: u32 = 1_000_000;
const MAX_SAMPLER_DESCRIPTOR_COUNT: u32 = 1000;
const MAX_RTV_DESCRIPTOR_COUNT: u32 = 1000;
const MAX_DSV_DESCRIPTOR_COUNT: u32 = 1000;

struct DescriptorHeap {
    heap: SendableIUnknown<ID3D12DescriptorHeap>,
    tail_handles: Mutex<(D3D12_CPU_DESCRIPTOR_HANDLE, u32)>,
    free_handles_queue: Mutex<VecDeque<(D3D12_CPU_DESCRIPTOR_HANDLE, u32)>>,
    increment_size: u32,
}

impl DescriptorHeap {
    fn new(heap: ID3D12DescriptorHeap, increment_size: u32) -> Self {
        let cpu_tail_handle = unsafe { heap.GetCPUDescriptorHandleForHeapStart() };

        Self {
            heap: heap.into(),
            tail_handles: Mutex::new((cpu_tail_handle, 0)),
            free_handles_queue: Default::default(),
            increment_size,
        }
    }

    fn allocate(&self) -> (D3D12_CPU_DESCRIPTOR_HANDLE, u32) {
        let mut cpu_queue = self.free_handles_queue.lock();
        if let Some(handle) = cpu_queue.pop_front() {
            handle
        } else {
            let mut tail_handle = self.tail_handles.lock();
            let handle = *tail_handle;
            tail_handle.0.ptr += self.increment_size as usize;
            tail_handle.1 += 1;
            handle
        }
    }

    fn free(&self, handles: (D3D12_CPU_DESCRIPTOR_HANDLE, u32)) {
        let mut cpu_queue = self.free_handles_queue.lock();
        cpu_queue.push_back(handles);
    }
}

pub struct DescriptorManager {
    cbv_srv_uav_heap: DescriptorHeap,
    sampler_heap: DescriptorHeap,
    rtv_heap: DescriptorHeap,
    dsv_heap: DescriptorHeap,
}

impl DescriptorManager {
    pub fn new(device: &ID3D12Device) -> Self {
        let cbv_srv_uav_heap = unsafe {
            let heap_desc = D3D12_DESCRIPTOR_HEAP_DESC {
                Type: D3D12_DESCRIPTOR_HEAP_TYPE_CBV_SRV_UAV,
                NumDescriptors: MAX_CBV_SRV_UAV_DESCRIPTOR_COUNT,
                Flags: D3D12_DESCRIPTOR_HEAP_FLAG_SHADER_VISIBLE,
                NodeMask: 0,
            };

            let heap: windows::core::Result<ID3D12DescriptorHeap> =
                device.CreateDescriptorHeap(&heap_desc);

            heap.unwrap()
        };

        let sampler_heap = unsafe {
            let heap_desc = D3D12_DESCRIPTOR_HEAP_DESC {
                Type: D3D12_DESCRIPTOR_HEAP_TYPE_SAMPLER,
                NumDescriptors: MAX_SAMPLER_DESCRIPTOR_COUNT,
                Flags: D3D12_DESCRIPTOR_HEAP_FLAG_SHADER_VISIBLE,
                NodeMask: 0,
            };

            let heap: windows::core::Result<ID3D12DescriptorHeap> =
                device.CreateDescriptorHeap(&heap_desc);
            heap.unwrap()
        };

        let rtv_heap = unsafe {
            let heap_desc = D3D12_DESCRIPTOR_HEAP_DESC {
                Type: D3D12_DESCRIPTOR_HEAP_TYPE_RTV,
                NumDescriptors: MAX_RTV_DESCRIPTOR_COUNT,
                Flags: D3D12_DESCRIPTOR_HEAP_FLAG_NONE,
                NodeMask: 0,
            };

            let heap: windows::core::Result<ID3D12DescriptorHeap> =
                device.CreateDescriptorHeap(&heap_desc);
            heap.unwrap()
        };

        let dsv_heap = unsafe {
            let heap_desc = D3D12_DESCRIPTOR_HEAP_DESC {
                Type: D3D12_DESCRIPTOR_HEAP_TYPE_DSV,
                NumDescriptors: MAX_DSV_DESCRIPTOR_COUNT,
                Flags: D3D12_DESCRIPTOR_HEAP_FLAG_NONE,
                NodeMask: 0,
            };

            let heap: windows::core::Result<ID3D12DescriptorHeap> =
                device.CreateDescriptorHeap(&heap_desc);
            heap.unwrap()
        };

        let cbv_srv_uav_increment_size = unsafe {
            device.GetDescriptorHandleIncrementSize(D3D12_DESCRIPTOR_HEAP_TYPE_CBV_SRV_UAV)
        };
        let sampler_increment_size =
            unsafe { device.GetDescriptorHandleIncrementSize(D3D12_DESCRIPTOR_HEAP_TYPE_SAMPLER) };
        let rtv_increment_size =
            unsafe { device.GetDescriptorHandleIncrementSize(D3D12_DESCRIPTOR_HEAP_TYPE_RTV) };
        let dsv_increment_size =
            unsafe { device.GetDescriptorHandleIncrementSize(D3D12_DESCRIPTOR_HEAP_TYPE_DSV) };

        utils::set_resource_name(&cbv_srv_uav_heap.clone().into(), "CBV/SRV/UAV Heap");
        utils::set_resource_name(&sampler_heap.clone().into(), "Sampler Heap");
        utils::set_resource_name(&rtv_heap.clone().into(), "RTV Heap");
        utils::set_resource_name(&dsv_heap.clone().into(), "DSV Heap");

        Self {
            cbv_srv_uav_heap: DescriptorHeap::new(cbv_srv_uav_heap, cbv_srv_uav_increment_size),
            sampler_heap: DescriptorHeap::new(sampler_heap, sampler_increment_size),
            rtv_heap: DescriptorHeap::new(rtv_heap, rtv_increment_size),
            dsv_heap: DescriptorHeap::new(dsv_heap, dsv_increment_size),
        }
    }

    pub fn cbv_srv_uav_heap(&self) -> &ID3D12DescriptorHeap {
        &self.cbv_srv_uav_heap.heap
    }

    pub fn sampler_heap(&self) -> &ID3D12DescriptorHeap {
        &self.sampler_heap.heap
    }

    pub fn free_cbv_srv_uav_descriptor_handle(&self, handles: (D3D12_CPU_DESCRIPTOR_HANDLE, u32)) {
        self.cbv_srv_uav_heap.free(handles);
    }

    pub fn free_rtv_descriptor_handle(&self, handles: (D3D12_CPU_DESCRIPTOR_HANDLE, u32)) {
        self.rtv_heap.free(handles);
    }

    pub fn free_sampler_descriptor_handle(&self, handles: (D3D12_CPU_DESCRIPTOR_HANDLE, u32)) {
        self.sampler_heap.free(handles);
    }

    pub fn allocate_cbv_srv_uav_descriptor_handle(&self) -> (D3D12_CPU_DESCRIPTOR_HANDLE, u32) {
        self.cbv_srv_uav_heap.allocate()
    }

    pub fn allocate_sampler_descriptor_handle(&self) -> (D3D12_CPU_DESCRIPTOR_HANDLE, u32) {
        self.sampler_heap.allocate()
    }

    pub fn allocate_rtv_descriptor_handle(&self) -> (D3D12_CPU_DESCRIPTOR_HANDLE, u32) {
        self.rtv_heap.allocate()
    }

    pub fn allocate_dsv_descriptor_handle(&self) -> (D3D12_CPU_DESCRIPTOR_HANDLE, u32) {
        self.dsv_heap.allocate()
    }
}
