use enumflags2::{bitflags, BitFlags};
use std::ptr::NonNull;
use std::{mem, ptr};
use windows::core::{Vtable, HRESULT};
use windows::Win32::Graphics::Direct3D12::*;
use windows::Win32::Graphics::Dxgi::*;
use ze_d3dmemoryallocator_sys::{
    D3D12MA_Allocation, D3D12MA_Allocation_ReleaseThis, D3D12MA_Allocator,
    D3D12MA_Allocator_CreatePool, D3D12MA_Allocator_CreateResource, D3D12MA_Allocator_ReleaseThis,
    D3D12MA_CreateAllocator, D3D12MA_Pool, D3D12MA_Pool_ReleaseThis, D3D12MA_ALLOCATION_DESC,
    D3D12MA_ALLOCATION_FLAGS_ALLOCATION_FLAG_CAN_ALIAS,
    D3D12MA_ALLOCATION_FLAGS_ALLOCATION_FLAG_COMMITTED,
    D3D12MA_ALLOCATION_FLAGS_ALLOCATION_FLAG_NEVER_ALLOCATE,
    D3D12MA_ALLOCATION_FLAGS_ALLOCATION_FLAG_STRATEGY_MIN_MEMORY,
    D3D12MA_ALLOCATION_FLAGS_ALLOCATION_FLAG_STRATEGY_MIN_OFFSET,
    D3D12MA_ALLOCATION_FLAGS_ALLOCATION_FLAG_STRATEGY_MIN_TIME,
    D3D12MA_ALLOCATION_FLAGS_ALLOCATION_FLAG_UPPER_ADDRESS,
    D3D12MA_ALLOCATION_FLAGS_ALLOCATION_FLAG_WITHIN_BUDGET, D3D12MA_ALLOCATOR_DESC,
    D3D12MA_POOL_DESC, D3D12MA_POOL_FLAGS_POOL_FLAG_ALGORITHM_LINEAR,
    D3D12MA_POOL_FLAGS_POOL_FLAG_MSAA_TEXTURES_ALWAYS_COMMITTED, IID,
};

#[repr(transparent)]
pub struct Allocator {
    allocator: NonNull<D3D12MA_Allocator>,
}

pub struct AllocatorDesc<'a> {
    pub device: &'a ID3D12Device,
    pub adapter: &'a IDXGIAdapter,
}

#[bitflags]
#[repr(u32)]
#[derive(Copy, Clone)]
pub enum AllocationFlagBits {
    Committed = D3D12MA_ALLOCATION_FLAGS_ALLOCATION_FLAG_COMMITTED as u32,
    NeverAllocate = D3D12MA_ALLOCATION_FLAGS_ALLOCATION_FLAG_NEVER_ALLOCATE as u32,
    WithinBudget = D3D12MA_ALLOCATION_FLAGS_ALLOCATION_FLAG_WITHIN_BUDGET as u32,
    UpperAddress = D3D12MA_ALLOCATION_FLAGS_ALLOCATION_FLAG_UPPER_ADDRESS as u32,
    CanAlias = D3D12MA_ALLOCATION_FLAGS_ALLOCATION_FLAG_CAN_ALIAS as u32,
    StrategyMinMemory = D3D12MA_ALLOCATION_FLAGS_ALLOCATION_FLAG_STRATEGY_MIN_MEMORY as u32,
    StrategyMinTime = D3D12MA_ALLOCATION_FLAGS_ALLOCATION_FLAG_STRATEGY_MIN_TIME as u32,
    StrategyMinOffset = D3D12MA_ALLOCATION_FLAGS_ALLOCATION_FLAG_STRATEGY_MIN_OFFSET as u32,
}
pub type AllocationFlags = BitFlags<AllocationFlagBits>;

pub struct AllocationDesc<'a> {
    pub flags: AllocationFlags,
    pub heap_type: D3D12_HEAP_TYPE,
    pub heap_flags: D3D12_HEAP_FLAGS,
    pub pool: Option<&'a Pool>,
}

#[bitflags]
#[repr(u32)]
#[derive(Copy, Clone)]
pub enum PoolFlagBits {
    Linear = D3D12MA_POOL_FLAGS_POOL_FLAG_ALGORITHM_LINEAR as u32,
    MsaaTexturesAlwaysCommitted =
        D3D12MA_POOL_FLAGS_POOL_FLAG_MSAA_TEXTURES_ALWAYS_COMMITTED as u32,
}
pub type PoolFlags = BitFlags<PoolFlagBits>;

pub struct PoolDesc {
    pub flags: PoolFlags,
    pub heap_properties: D3D12_HEAP_PROPERTIES,
    pub heap_flags: D3D12_HEAP_FLAGS,
}

impl Allocator {
    pub fn new(desc: AllocatorDesc) -> Result<Allocator, HRESULT> {
        let mut allocator = ptr::null_mut();

        let result = unsafe {
            let d3d_desc = D3D12MA_ALLOCATOR_DESC {
                Flags: 0,
                pDevice: desc.device.as_raw() as *mut _,
                PreferredBlockSize: 0,
                pAllocationCallbacks: ptr::null_mut(),
                pAdapter: desc.adapter.as_raw() as *mut _,
            };

            D3D12MA_CreateAllocator(&d3d_desc, &mut allocator)
        };
        if result == 0 {
            Ok(Allocator {
                allocator: unsafe { NonNull::new_unchecked(allocator) },
            })
        } else {
            Err(HRESULT(result))
        }
    }

    pub fn create_resource(
        &self,
        allocation_desc: &AllocationDesc,
        resource_desc: &D3D12_RESOURCE_DESC,
    ) -> Result<Allocation, HRESULT> {
        let mut allocation = ptr::null_mut();

        let result = unsafe {
            let alloc_desc = D3D12MA_ALLOCATION_DESC {
                Flags: allocation_desc.flags.bits() as i32,
                HeapType: mem::transmute(allocation_desc.heap_type),
                ExtraHeapFlags: mem::transmute(allocation_desc.heap_flags),
                CustomPool: if let Some(pool) = allocation_desc.pool {
                    pool.pool.as_ptr()
                } else {
                    ptr::null_mut()
                },
                pPrivateData: ptr::null_mut(),
            };

            D3D12MA_Allocator_CreateResource(
                self.allocator.as_ptr(),
                &alloc_desc,
                resource_desc as *const _ as *mut _,
                0,
                ptr::null(),
                &mut allocation,
                &IID {
                    Data1: 0,
                    Data2: 0,
                    Data3: 0,
                    Data4: [0; 8],
                },
                ptr::null_mut(),
            )
        };

        if result == 0 {
            Ok(Allocation {
                allocation: unsafe { NonNull::new_unchecked(allocation) },
            })
        } else {
            Err(HRESULT(result))
        }
    }

    pub fn create_pool(&self, desc: &PoolDesc) -> Result<Pool, HRESULT> {
        let mut pool = ptr::null_mut();

        let result = unsafe {
            D3D12MA_Allocator_CreatePool(
                self.allocator.as_ptr(),
                &D3D12MA_POOL_DESC {
                    Flags: desc.flags.bits() as i32,
                    HeapProperties: mem::transmute(desc.heap_properties),
                    HeapFlags: mem::transmute(desc.heap_flags),
                    BlockSize: 0,
                    MinBlockCount: 0,
                    MaxBlockCount: 0,
                    MinAllocationAlignment: 0,
                    pProtectedSession: ptr::null_mut(),
                },
                &mut pool,
            )
        };

        if result == 0 {
            Ok(Pool {
                pool: unsafe { NonNull::new_unchecked(pool) },
            })
        } else {
            Err(HRESULT(result))
        }
    }
}

impl Drop for Allocator {
    fn drop(&mut self) {
        unsafe { D3D12MA_Allocator_ReleaseThis(self.allocator.as_ptr() as *mut _) };
    }
}

// SAFETY: D3D12MA::Allocator is thread-safe unless D3D12MA::ALLOCATOR_FLAG_SINGLETHREADED is provided
// which this crate don't allow
unsafe impl Send for Allocator {}
unsafe impl Sync for Allocator {}

#[repr(transparent)]
pub struct Allocation {
    allocation: NonNull<D3D12MA_Allocation>,
}

unsafe impl Send for Allocation {}
unsafe impl Sync for Allocation {}

impl Allocation {
    pub fn resource(&self) -> Option<&ID3D12Resource> {
        let allocation = unsafe { self.allocation.as_ref() };
        if allocation.m_Resource.is_null() {
            None
        } else {
            Some(unsafe {
                (&allocation.m_Resource as *const _ as *mut ID3D12Resource)
                    .as_ref()
                    .unwrap_unchecked()
            })
        }
    }
}

impl Drop for Allocation {
    fn drop(&mut self) {
        unsafe { D3D12MA_Allocation_ReleaseThis(self.allocation.as_ptr() as *mut _) };
    }
}

#[repr(transparent)]
pub struct Pool {
    pool: NonNull<D3D12MA_Pool>,
}

impl Drop for Pool {
    fn drop(&mut self) {
        unsafe { D3D12MA_Pool_ReleaseThis(self.pool.as_ptr() as *mut _) };
    }
}

unsafe impl Send for Pool {}
unsafe impl Sync for Pool {}
