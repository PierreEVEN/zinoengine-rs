pub(crate) mod buffer;
pub(crate) mod cmd_list;
mod memory_pool;
pub(crate) mod sampler;
pub(crate) mod shader;
pub(crate) mod swapchain;
pub(crate) mod texture;
pub(crate) mod views;

use crate::descriptor_manager::DescriptorManager;
use crate::device::buffer::D3D12Buffer;
use crate::device::cmd_list::{D3D12CommandList, D3D12CommandListPipelineType};
use crate::device::sampler::D3D12Sampler;
use crate::device::shader::D3D12ShaderModule;
use crate::device::swapchain::D3D12SwapChain;
use crate::device::texture::D3D12Texture;
use crate::device::views::{D3D12DepthStencilView, D3D12RenderTargetView, D3D12ShaderResourceView};
use crate::frame_manager::FrameManager;
use crate::pipeline_manager::{GraphicsPipelineStateDesc, PipelineManager};
#[cfg(feature = "pix")]
use crate::pix::{pix_begin_event_cmd_list, pix_end_event_cmd_list};
use crate::utils::*;
use parking_lot::Mutex;
use raw_window_handle::RawWindowHandle;
use std::ffi::c_void;
use std::mem::{size_of, ManuallyDrop};
use std::ops::Deref;
use std::slice;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use windows::core::Interface;
use windows::Win32::Foundation::{BOOL, HWND, RECT};
use windows::Win32::Graphics::Direct3D::{ID3DBlob, D3D_PRIMITIVE_TOPOLOGY_TRIANGLELIST};
use windows::Win32::Graphics::Direct3D12::*;
use windows::Win32::Graphics::Dxgi::Common::*;
use windows::Win32::Graphics::Dxgi::*;

use crate::device::memory_pool::D3D12MemoryPool;
use ze_core::color::Color4f32;
use ze_core::maths::RectI32;
use ze_d3dmemoryallocator::{
    AllocationDesc, Allocator, AllocatorDesc, PoolDesc, PoolFlagBits, PoolFlags,
};
use ze_gfx::backend::*;
use ze_gfx::ShaderStageFlagBits;

pub(crate) struct D3D12Device {
    dxgi_factory: Arc<Mutex<SendableIUnknown<IDXGIFactory4>>>,
    device: SendableIUnknown<ID3D12Device2>,
    graphics_queue: SendableIUnknown<ID3D12CommandQueue>,
    _compute_queue: SendableIUnknown<ID3D12CommandQueue>,
    _transfer_queue: SendableIUnknown<ID3D12CommandQueue>,
    frame_manager: Arc<FrameManager>,
    descriptor_manager: Arc<DescriptorManager>,
    pipeline_manager: PipelineManager,
    default_root_signature: SendableIUnknown<ID3D12RootSignature>,
    frame_index: AtomicU64,
    transient_memory_pool: MemoryPool,
    allocator: Arc<Allocator>,
}

impl D3D12Device {
    pub fn new(
        dxgi_factory: Arc<Mutex<SendableIUnknown<IDXGIFactory4>>>,
        device: SendableIUnknown<ID3D12Device>,
        adapter: IDXGIAdapter1,
    ) -> Self {
        let graphics_queue: ID3D12CommandQueue = {
            unsafe {
                device.CreateCommandQueue(&D3D12_COMMAND_QUEUE_DESC {
                    Type: D3D12_COMMAND_LIST_TYPE_DIRECT,
                    Priority: 0,
                    Flags: Default::default(),
                    NodeMask: 0,
                })
            }
        }
        .unwrap();

        let compute_queue: ID3D12CommandQueue = {
            unsafe {
                device.CreateCommandQueue(&D3D12_COMMAND_QUEUE_DESC {
                    Type: D3D12_COMMAND_LIST_TYPE_COMPUTE,
                    Priority: 0,
                    Flags: Default::default(),
                    NodeMask: 0,
                })
            }
        }
        .unwrap();

        let transfer_queue: ID3D12CommandQueue = {
            unsafe {
                device.CreateCommandQueue(&D3D12_COMMAND_QUEUE_DESC {
                    Type: D3D12_COMMAND_LIST_TYPE_COPY,
                    Priority: 0,
                    Flags: Default::default(),
                    NodeMask: 0,
                })
            }
        }
        .unwrap();

        set_resource_name(&graphics_queue.clone().into(), "Graphics Queue");
        set_resource_name(&compute_queue.clone().into(), "Compute Queue");
        set_resource_name(&transfer_queue.clone().into(), "Transfer Queue");

        let allocator = Arc::new(
            Allocator::new(AllocatorDesc {
                device: &*device,
                adapter: (&adapter).into(),
            })
            .expect("Failed to create allocator"),
        );

        let default_root_signature: ID3D12RootSignature = {
            let parameters = [D3D12_ROOT_PARAMETER1 {
                ParameterType: D3D12_ROOT_PARAMETER_TYPE_32BIT_CONSTANTS,
                Anonymous: D3D12_ROOT_PARAMETER1_0 {
                    Constants: D3D12_ROOT_CONSTANTS {
                        ShaderRegister: 0,
                        RegisterSpace: 0,
                        Num32BitValues: 32,
                    },
                },
                ShaderVisibility: D3D12_SHADER_VISIBILITY_ALL,
            }];

            let desc = D3D12_VERSIONED_ROOT_SIGNATURE_DESC {
                Version: D3D_ROOT_SIGNATURE_VERSION_1_1,
                Anonymous: D3D12_VERSIONED_ROOT_SIGNATURE_DESC_0 {
                    Desc_1_1: D3D12_ROOT_SIGNATURE_DESC1 {
                        NumParameters: parameters.len() as u32,
                        pParameters: parameters.as_ptr(),
                        NumStaticSamplers: 0,
                        pStaticSamplers: std::ptr::null(),
                        Flags: D3D12_ROOT_SIGNATURE_FLAG_CBV_SRV_UAV_HEAP_DIRECTLY_INDEXED
                            | D3D12_ROOT_SIGNATURE_FLAG_SAMPLER_HEAP_DIRECTLY_INDEXED,
                    },
                },
            };

            unsafe {
                let mut blob: Option<ID3DBlob> = None;
                D3D12SerializeVersionedRootSignature(&desc, &mut blob, None).unwrap();
                let blob = blob.unwrap();
                let ptr = blob.GetBufferPointer() as *const u8;
                device
                    .CreateRootSignature(0, slice::from_raw_parts(ptr, blob.GetBufferSize()))
                    .unwrap()
            }
        };

        let device: ID3D12Device2 = device.cast().unwrap();
        let transient_memory_pool = D3D12MemoryPool {
            pool: allocator
                .create_pool(&PoolDesc {
                    flags: PoolFlags::from(PoolFlagBits::Linear),
                    heap_properties: D3D12_HEAP_PROPERTIES {
                        Type: D3D12_HEAP_TYPE_DEFAULT,
                        CPUPageProperty: D3D12_CPU_PAGE_PROPERTY_UNKNOWN,
                        MemoryPoolPreference: D3D12_MEMORY_POOL_UNKNOWN,
                        CreationNodeMask: 0,
                        VisibleNodeMask: 0,
                    },
                    heap_flags: Default::default(),
                })
                .unwrap(),
        };

        Self {
            dxgi_factory,
            device: device.clone().into(),
            frame_manager: Arc::new(FrameManager::new(
                2,
                &device,
                &graphics_queue,
                &compute_queue,
                &transfer_queue,
            )),
            descriptor_manager: Arc::new(DescriptorManager::new(&device)),
            pipeline_manager: PipelineManager::default(),
            default_root_signature: default_root_signature.into(),
            graphics_queue: SendableIUnknown(graphics_queue),
            _compute_queue: SendableIUnknown(compute_queue),
            _transfer_queue: SendableIUnknown(transfer_queue),
            frame_index: AtomicU64::new(0),
            transient_memory_pool: MemoryPool::new(Box::new(transient_memory_pool)),
            allocator,
        }
    }

    fn flush_pipeline_state(&self, command_list: &mut D3D12CommandList) {
        if command_list.pipeline_state_dirty {
            match &mut command_list.pipeline {
                D3D12CommandListPipelineType::Graphics(desc) => {
                    // Apply render pass parameters to desc
                    *desc.rtv_formats = D3D12_RT_FORMAT_ARRAY {
                        RTFormats: command_list.render_pass_rtv_formats,
                        NumRenderTargets: command_list.render_pass_rt_count,
                    };
                    *desc.dsv_format = command_list.render_pass_dsv_format;

                    let pipeline = self
                        .pipeline_manager
                        .get_or_create_graphics_pipeline(&self.device, desc);
                    unsafe {
                        command_list.cmd_list.SetPipelineState(&pipeline);
                    }
                }
                D3D12CommandListPipelineType::Compute(_) => todo!(),
                _ => {}
            }

            command_list.pipeline_state_dirty = false;
        }
    }

    pub fn device(&self) -> &SendableIUnknown<ID3D12Device2> {
        &self.device
    }
}

impl Drop for D3D12Device {
    fn drop(&mut self) {
        self.wait_idle();
    }
}

impl Device for D3D12Device {
    fn begin_frame(&self) {
        let old_count = self.frame_index.fetch_add(1, Ordering::SeqCst);

        if old_count > 0 {
            self.frame_manager.begin_frame(self);
        }
    }

    fn end_frame(&self) {}

    fn create_buffer(
        &self,
        info: &BufferDesc,
        memory_pool: Option<&MemoryPool>,
        name: &str,
    ) -> Result<Buffer, DeviceError> {
        let mut flags = D3D12_RESOURCE_FLAGS::default();
        if info.usage.contains(BufferUsageFlagBits::UnorderedAccess) {
            flags |= D3D12_RESOURCE_FLAG_ALLOW_UNORDERED_ACCESS;
        }

        let buffer_desc = D3D12_RESOURCE_DESC {
            Dimension: D3D12_RESOURCE_DIMENSION_BUFFER,
            Alignment: D3D12_DEFAULT_RESOURCE_PLACEMENT_ALIGNMENT as u64,
            Width: info.size_bytes,
            Height: 1,
            DepthOrArraySize: 1,
            MipLevels: 1,
            Format: DXGI_FORMAT_UNKNOWN,
            SampleDesc: DXGI_SAMPLE_DESC {
                Count: 1,
                Quality: 0,
            },
            Layout: D3D12_TEXTURE_LAYOUT_ROW_MAJOR,
            Flags: flags,
        };

        let allocation_desc = AllocationDesc {
            flags: Default::default(),
            heap_type: get_heap_type_from_memory_location(info.memory_desc.memory_location),
            heap_flags: Default::default(),
            pool: if let Some(pool) = memory_pool {
                Some(
                    &pool
                        .backend_data
                        .downcast_ref::<D3D12MemoryPool>()
                        .unwrap()
                        .pool,
                )
            } else {
                None
            },
        };

        match self
            .allocator
            .create_resource(&allocation_desc, &buffer_desc)
        {
            Ok(allocation) => {
                let resource = allocation.resource().unwrap();
                let mapped_ptr = {
                    if info.memory_desc.memory_location == MemoryLocation::CpuToGpu {
                        unsafe {
                            let mut mapped_ptr = std::ptr::null_mut();
                            let range = D3D12_RANGE { Begin: 0, End: 0 };
                            resource
                                .Map(0, Some(&range), Some(&mut mapped_ptr))
                                .unwrap();
                            let mapped_ptr = mapped_ptr.cast::<u8>();
                            Some(mapped_ptr)
                        }
                    } else {
                        None
                    }
                };

                let gpu_virtual_address = unsafe { resource.GetGPUVirtualAddress() };

                {
                    let resource = resource.clone().into();
                    set_resource_name(&resource, name);
                }

                Ok(Buffer::new(
                    info,
                    Box::new(D3D12Buffer::new(
                        self.frame_manager.clone(),
                        resource.clone().into(),
                        Some(allocation),
                        mapped_ptr,
                        gpu_virtual_address,
                    )),
                ))
            }
            Err(err) => Err(convert_d3d_error_to_ze_device_error(err.into())),
        }
    }

    fn create_texture(
        &self,
        info: &TextureDesc,
        memory_pool: Option<&MemoryPool>,
        name: &str,
    ) -> Result<Texture, DeviceError> {
        let mut flags = D3D12_RESOURCE_FLAGS::default();
        if info
            .usage_flags
            .contains(TextureUsageFlagBits::UnorderedAccess)
        {
            flags |= D3D12_RESOURCE_FLAG_ALLOW_UNORDERED_ACCESS;
        }

        if info
            .usage_flags
            .contains(TextureUsageFlagBits::RenderTarget)
        {
            flags |= D3D12_RESOURCE_FLAG_ALLOW_RENDER_TARGET;
        }

        if info
            .usage_flags
            .contains(TextureUsageFlagBits::DepthStencil)
        {
            flags |= D3D12_RESOURCE_FLAG_ALLOW_DEPTH_STENCIL;
        }

        let dimension = {
            if info.depth > 1 {
                D3D12_RESOURCE_DIMENSION_TEXTURE3D
            } else if info.height > 0 {
                D3D12_RESOURCE_DIMENSION_TEXTURE2D
            } else {
                D3D12_RESOURCE_DIMENSION_TEXTURE1D
            }
        };

        let texture_desc = D3D12_RESOURCE_DESC {
            Dimension: dimension,
            Alignment: 0,
            Width: info.width as u64,
            Height: info.height,
            DepthOrArraySize: info.depth as u16,
            MipLevels: info.mip_levels as u16,
            Format: get_dxgi_format_from_ze_format(info.format),
            SampleDesc: get_dxgi_sample_desc_from_ze_sample_desc(info.sample_desc),
            Layout: D3D12_TEXTURE_LAYOUT_UNKNOWN,
            Flags: flags,
        };

        let allocation_desc = AllocationDesc {
            flags: Default::default(),
            heap_type: get_heap_type_from_memory_location(info.memory_desc.memory_location),
            heap_flags: Default::default(),
            pool: if let Some(pool) = memory_pool {
                Some(
                    &pool
                        .backend_data
                        .downcast_ref::<D3D12MemoryPool>()
                        .unwrap()
                        .pool,
                )
            } else {
                None
            },
        };

        match self
            .allocator
            .create_resource(&allocation_desc, &texture_desc)
        {
            Ok(allocation) => {
                let resource = allocation.resource().unwrap();
                {
                    let resource = resource.clone().into();
                    set_resource_name(&resource, name);
                }

                Ok(Texture::new(
                    *info,
                    Box::new(D3D12Texture::new(
                        self.frame_manager.clone(),
                        resource.clone().into(),
                        Some(allocation),
                    )),
                ))
            }
            Err(err) => Err(convert_d3d_error_to_ze_device_error(err.into())),
        }
    }

    fn create_shader_resource_view(
        &self,
        desc: &ShaderResourceViewDesc,
    ) -> Result<ShaderResourceView, DeviceError> {
        let (resource, d3d_desc) = match desc {
            ShaderResourceViewDesc::Buffer(buffer) => {
                let buffer_size = buffer.buffer.info.size_bytes;
                let d3d_buffer_srv = match &buffer.ty {
                    BufferSRVType::Raw(raw) => D3D12_BUFFER_SRV {
                        FirstElement: (raw.offset_in_bytes / 4) as u64,
                        NumElements: (buffer.buffer.info.size_bytes / 4) as u32,
                        StructureByteStride: 0,
                        Flags: D3D12_BUFFER_SRV_FLAG_RAW,
                    },
                    BufferSRVType::Structured(structured) => D3D12_BUFFER_SRV {
                        FirstElement: structured.offset_in_bytes
                            / structured.stride_in_bytes as u64,
                        NumElements: buffer_size.min(buffer_size - structured.offset_in_bytes)
                            as u32
                            / structured.stride_in_bytes,
                        StructureByteStride: structured.stride_in_bytes,
                        Flags: D3D12_BUFFER_SRV_FLAG_NONE,
                    },
                };

                let format = match buffer.ty {
                    BufferSRVType::Raw(_) => DXGI_FORMAT_R32_TYPELESS,
                    BufferSRVType::Structured(_) => DXGI_FORMAT_UNKNOWN,
                };

                let d3d_desc = D3D12_SHADER_RESOURCE_VIEW_DESC {
                    Format: format,
                    Shader4ComponentMapping: D3D12_DEFAULT_SHADER_4_COMPONENT_MAPPING,
                    ViewDimension: D3D12_SRV_DIMENSION_BUFFER,
                    Anonymous: D3D12_SHADER_RESOURCE_VIEW_DESC_0 {
                        Buffer: d3d_buffer_srv,
                    },
                };

                (
                    buffer
                        .buffer
                        .backend_data
                        .downcast_ref::<D3D12Buffer>()
                        .unwrap()
                        .resource
                        .deref(),
                    d3d_desc,
                )
            }
            ShaderResourceViewDesc::Texture2D(texture) => {
                let d3d_desc = D3D12_SHADER_RESOURCE_VIEW_DESC {
                    Format: get_dxgi_format_from_ze_format(texture.format),
                    Shader4ComponentMapping: D3D12_DEFAULT_SHADER_4_COMPONENT_MAPPING,
                    ViewDimension: D3D12_SRV_DIMENSION_TEXTURE2D,
                    Anonymous: D3D12_SHADER_RESOURCE_VIEW_DESC_0 {
                        Texture2D: D3D12_TEX2D_SRV {
                            MostDetailedMip: texture.min_mip_level,
                            MipLevels: texture.mip_levels,
                            PlaneSlice: 0,
                            ResourceMinLODClamp: 0.0,
                        },
                    },
                };

                (
                    texture
                        .texture
                        .backend_data
                        .downcast_ref::<D3D12Texture>()
                        .unwrap()
                        .texture
                        .deref(),
                    d3d_desc,
                )
            }
        };

        let handle = self
            .descriptor_manager
            .allocate_cbv_srv_uav_descriptor_handle();
        unsafe {
            self.device
                .CreateShaderResourceView(resource, Some(&d3d_desc), handle.0)
        }

        Ok(ShaderResourceView::new(
            desc.clone(),
            Box::new(D3D12ShaderResourceView {
                descriptor_manager: self.descriptor_manager.clone(),
                handle,
            }),
        ))
    }

    fn create_render_target_view(
        &self,
        desc: &RenderTargetViewDesc,
    ) -> Result<RenderTargetView, DeviceError> {
        let mut d3d_desc = D3D12_RENDER_TARGET_VIEW_DESC {
            Format: get_dxgi_format_from_ze_format(desc.format),
            ViewDimension: Default::default(),
            Anonymous: Default::default(),
        };

        let resource = desc
            .resource
            .backend_data
            .downcast_ref::<D3D12Texture>()
            .unwrap()
            .texture
            .deref();

        match &desc.ty {
            RenderTargetViewType::Texture2D(info) => {
                d3d_desc.ViewDimension = D3D12_RTV_DIMENSION_TEXTURE2D;
                d3d_desc.Anonymous = D3D12_RENDER_TARGET_VIEW_DESC_0 {
                    Texture2D: D3D12_TEX2D_RTV {
                        MipSlice: info.mip_level,
                        PlaneSlice: 0,
                    },
                };
            }
        }

        let handle = self.descriptor_manager.allocate_rtv_descriptor_handle();
        unsafe {
            self.device
                .CreateRenderTargetView(resource, Some(&d3d_desc), handle.0)
        }

        Ok(RenderTargetView::new(
            desc.clone(),
            Box::new(D3D12RenderTargetView {
                descriptor_manager: self.descriptor_manager.clone(),
                handle,
            }),
        ))
    }

    fn create_depth_stencil_view(
        &self,
        desc: &DepthStencilViewDesc,
    ) -> Result<DepthStencilView, DeviceError> {
        let mut d3d_desc = D3D12_DEPTH_STENCIL_VIEW_DESC {
            Format: get_dxgi_format_from_ze_format(desc.format),
            ViewDimension: Default::default(),
            Flags: Default::default(),
            Anonymous: Default::default(),
        };

        let resource = desc
            .resource
            .backend_data
            .downcast_ref::<D3D12Texture>()
            .unwrap()
            .texture
            .deref();

        match &desc.ty {
            DepthStencilViewType::Texture2D(info) => {
                d3d_desc.ViewDimension = D3D12_DSV_DIMENSION_TEXTURE2D;
                d3d_desc.Anonymous = D3D12_DEPTH_STENCIL_VIEW_DESC_0 {
                    Texture2D: D3D12_TEX2D_DSV {
                        MipSlice: info.mip_level,
                    },
                };
            }
        }

        let handle = self.descriptor_manager.allocate_dsv_descriptor_handle();
        unsafe {
            self.device
                .CreateDepthStencilView(resource, Some(&d3d_desc), handle.0)
        }

        Ok(DepthStencilView::new(
            desc.clone(),
            Box::new(D3D12DepthStencilView {
                descriptor_manager: self.descriptor_manager.clone(),
                handle,
            }),
        ))
    }

    fn create_swapchain(
        &self,
        info: &SwapChainDesc,
        old_swapchain: Option<SwapChain>,
    ) -> Result<SwapChain, DeviceError> {
        let swapchain_buffer_count = std::cmp::max(2, self.frame_manager.frame_count());

        if let Some(old_swapchain) = old_swapchain {
            let swapchain = old_swapchain
                .backend_data
                .downcast_ref::<D3D12SwapChain>()
                .unwrap()
                .swapchain
                .clone();

            drop(old_swapchain);

            self.frame_manager
                .current_frame()
                .resource_manager()
                .flush();

            unsafe {
                swapchain
                    .ResizeBuffers(
                        swapchain_buffer_count as u32,
                        info.width,
                        info.height,
                        get_dxgi_format_from_ze_format(info.format),
                        0,
                    )
                    .unwrap();
            };

            let mut textures = Vec::with_capacity(swapchain_buffer_count);
            for i in 0..swapchain_buffer_count {
                let buffer: ID3D12Resource =
                    unsafe { swapchain.GetBuffer::<ID3D12Resource>(i as u32) }.unwrap();
                let d3d_desc: D3D12_RESOURCE_DESC = unsafe { buffer.GetDesc() };

                let desc = TextureDesc {
                    width: d3d_desc.Width as u32,
                    height: d3d_desc.Height as u32,
                    depth: d3d_desc.DepthOrArraySize as u32,
                    mip_levels: d3d_desc.MipLevels as u32,
                    format: get_ze_format_from_dxgi_format(d3d_desc.Format),
                    sample_desc: get_ze_sample_desc_from_dxgi_sample_desc(d3d_desc.SampleDesc),
                    usage_flags: info.usage_flags,
                    memory_desc: MemoryDesc {
                        memory_location: MemoryLocation::GpuOnly,
                        memory_flags: Default::default(),
                    },
                };

                set_resource_name(&buffer.clone().into(), &format!("Swapchain Texture {}", i));

                let texture = Texture::new(
                    desc,
                    Box::new(D3D12Texture::new(
                        self.frame_manager.clone(),
                        buffer.into(),
                        None,
                    )),
                );
                textures.push(Arc::new(texture));
            }

            Ok(SwapChain::new(
                *info,
                Box::new(D3D12SwapChain::new(swapchain.0.into(), textures)),
            ))
        } else if let RawWindowHandle::Win32(hwnd) = info.window_handle {
            let desc = DXGI_SWAP_CHAIN_DESC1 {
                Width: info.width,
                Height: info.height,
                Format: get_dxgi_format_from_ze_format(info.format),
                Stereo: BOOL::from(false),
                SampleDesc: get_dxgi_sample_desc_from_ze_sample_desc(info.sample_desc),
                BufferUsage: DXGI_USAGE_RENDER_TARGET_OUTPUT,
                BufferCount: swapchain_buffer_count as u32,
                Scaling: DXGI_SCALING_STRETCH,
                SwapEffect: DXGI_SWAP_EFFECT_FLIP_DISCARD,
                AlphaMode: DXGI_ALPHA_MODE_UNSPECIFIED,
                Flags: 0,
            };

            let factory = self.dxgi_factory.lock();
            let swapchain = unsafe {
                factory.CreateSwapChainForHwnd(
                    self.graphics_queue.deref(),
                    HWND(hwnd.hwnd as isize),
                    &desc,
                    None,
                    None,
                )
            };

            match swapchain {
                Ok(swapchain) => {
                    let swapchain: IDXGISwapChain3 = swapchain.cast::<IDXGISwapChain3>().unwrap();

                    let mut textures = Vec::with_capacity(swapchain_buffer_count);
                    for i in 0..swapchain_buffer_count {
                        let buffer: ID3D12Resource =
                            unsafe { swapchain.GetBuffer::<ID3D12Resource>(i as u32) }.unwrap();
                        let d3d_desc: D3D12_RESOURCE_DESC = unsafe { buffer.GetDesc() };

                        let desc = TextureDesc {
                            width: d3d_desc.Width as u32,
                            height: d3d_desc.Height as u32,
                            depth: d3d_desc.DepthOrArraySize as u32,
                            mip_levels: d3d_desc.MipLevels as u32,
                            format: get_ze_format_from_dxgi_format(d3d_desc.Format),
                            sample_desc: get_ze_sample_desc_from_dxgi_sample_desc(
                                d3d_desc.SampleDesc,
                            ),
                            usage_flags: info.usage_flags,
                            memory_desc: MemoryDesc {
                                memory_location: MemoryLocation::GpuOnly,
                                memory_flags: Default::default(),
                            },
                        };

                        set_resource_name(
                            &buffer.clone().into(),
                            &format!("Swapchain Texture {}", i),
                        );

                        let texture = Texture::new(
                            desc,
                            Box::new(D3D12Texture::new(
                                self.frame_manager.clone(),
                                buffer.into(),
                                None,
                            )),
                        );
                        textures.push(Arc::new(texture));
                    }
                    Ok(SwapChain::new(
                        *info,
                        Box::new(D3D12SwapChain::new(swapchain.into(), textures)),
                    ))
                }
                Err(err) => Err(convert_d3d_error_to_ze_device_error(err)),
            }
        } else {
            Err(DeviceError::Unknown)
        }
    }

    fn create_shader_module(&self, bytecode: &[u8]) -> Result<ShaderModule, DeviceError> {
        Ok(ShaderModule::new(Box::new(D3D12ShaderModule::new(
            Vec::from(bytecode),
        ))))
    }

    fn create_command_list(&self, queue_type: QueueType) -> Result<CommandList, DeviceError> {
        let (_, cmd_list) = self
            .frame_manager
            .current_frame()
            .command_manager()
            .create_command_list(self, queue_type);

        let cmd_list = D3D12CommandList::new(cmd_list);

        if queue_type != QueueType::Transfer {
            unsafe {
                let heaps = [
                    Some(self.descriptor_manager.cbv_srv_uav_heap().clone()),
                    Some(self.descriptor_manager.sampler_heap().clone()),
                ];
                cmd_list.cmd_list.SetDescriptorHeaps(&heaps);
                cmd_list
                    .cmd_list
                    .SetGraphicsRootSignature(&*self.default_root_signature);
                cmd_list
                    .cmd_list
                    .SetComputeRootSignature(&*self.default_root_signature);
                cmd_list
                    .cmd_list
                    .IASetPrimitiveTopology(D3D_PRIMITIVE_TOPOLOGY_TRIANGLELIST);
            }
        }

        Ok(CommandList::new(Box::new(cmd_list)))
    }

    fn create_sampler(&self, desc: &SamplerDesc) -> Result<Sampler, DeviceError> {
        let handle = self.descriptor_manager.allocate_sampler_descriptor_handle();
        unsafe {
            self.device.CreateSampler(
                &D3D12_SAMPLER_DESC {
                    Filter: get_d3d_filter_from_ze_filter(desc.filter),
                    AddressU: get_d3d_texture_address_mode_from_ze_texture_address_mode(
                        desc.address_u,
                    ),
                    AddressV: get_d3d_texture_address_mode_from_ze_texture_address_mode(
                        desc.address_v,
                    ),
                    AddressW: get_d3d_texture_address_mode_from_ze_texture_address_mode(
                        desc.address_w,
                    ),
                    MipLODBias: desc.mip_lod_bias,
                    MaxAnisotropy: desc.max_anisotropy,
                    ComparisonFunc: get_d3d_compare_func_from_ze_compare_op(desc.compare_op),
                    BorderColor: [0.0, 0.0, 0.0, 1.0],
                    MinLOD: desc.min_lod,
                    MaxLOD: desc.max_lod,
                },
                handle.0,
            );
        }

        Ok(Sampler::new(
            desc.clone(),
            Box::new(D3D12Sampler {
                descriptor_manager: self.descriptor_manager.clone(),
                handle,
            }),
        ))
    }

    fn buffer_mapped_ptr(&self, buffer: &Buffer) -> Option<*mut u8> {
        let buffer = unsafe {
            buffer
                .backend_data
                .downcast_ref::<D3D12Buffer>()
                .unwrap_unchecked()
        };

        buffer.mapped_ptr
    }

    fn texture_subresource_layout(
        &self,
        texture: &Texture,
        subresource_index: u32,
    ) -> TextureSubresourceLayout {
        let texture = unsafe {
            texture
                .backend_data
                .downcast_ref::<D3D12Texture>()
                .unwrap_unchecked()
        };

        let mut footprint = D3D12_PLACED_SUBRESOURCE_FOOTPRINT::default();

        let mut total_bytes = 0;
        unsafe {
            let mut num_rows = 0;
            let mut row_size_in_bytes = 0;

            self.device.GetCopyableFootprints(
                &texture.texture.GetDesc(),
                subresource_index,
                1,
                0,
                Some(&mut footprint),
                Some(&mut num_rows),
                Some(&mut row_size_in_bytes),
                Some(&mut total_bytes),
            );
        }

        TextureSubresourceLayout {
            offset_in_bytes: footprint.Offset,
            row_pitch_in_bytes: footprint.Footprint.RowPitch as u64,
            size_in_bytes: total_bytes,
        }
    }

    fn swapchain_backbuffer_count(&self, swapchain: &SwapChain) -> usize {
        let swapchain = unsafe {
            swapchain
                .backend_data
                .downcast_ref::<D3D12SwapChain>()
                .unwrap_unchecked()
        };

        swapchain.textures.len()
    }

    fn swapchain_backbuffer_index(&self, swapchain: &SwapChain) -> u32 {
        let swapchain = unsafe {
            swapchain
                .backend_data
                .downcast_ref::<D3D12SwapChain>()
                .unwrap_unchecked()
        };

        unsafe { swapchain.swapchain.GetCurrentBackBufferIndex() }
    }

    fn swapchain_backbuffer(
        &self,
        swapchain: &SwapChain,
        index: u32,
    ) -> Result<Arc<Texture>, DeviceError> {
        let swapchain = unsafe {
            swapchain
                .backend_data
                .downcast_ref::<D3D12SwapChain>()
                .unwrap_unchecked()
        };

        Ok(swapchain.textures[index as usize].clone())
    }

    fn present(&self, swapchain: &SwapChain) {
        let swapchain = unsafe {
            swapchain
                .backend_data
                .downcast_ref::<D3D12SwapChain>()
                .unwrap_unchecked()
        };
        unsafe {
            let mut flags = 0;
            if swapchain.need_restart.load(Ordering::SeqCst) {
                flags |= DXGI_PRESENT_RESTART;
                swapchain.need_restart.store(false, Ordering::SeqCst);
            }
            swapchain.swapchain.Present(0, flags).unwrap();
        }
    }

    fn transient_memory_pool(&self) -> &MemoryPool {
        &self.transient_memory_pool
    }

    fn cmd_copy_buffer_regions(
        &self,
        cmd_list: &mut CommandList,
        src_buffer: &Buffer,
        dst_buffer: &Buffer,
        regions: &[BufferCopyRegion],
    ) {
        let cmd_list = unsafe {
            cmd_list
                .backend_data
                .downcast_ref::<D3D12CommandList>()
                .unwrap_unchecked()
        };

        let src_buffer = unsafe {
            src_buffer
                .backend_data
                .downcast_ref::<D3D12Buffer>()
                .unwrap_unchecked()
        };

        let dst_buffer = unsafe {
            dst_buffer
                .backend_data
                .downcast_ref::<D3D12Buffer>()
                .unwrap_unchecked()
        };

        for region in regions {
            unsafe {
                cmd_list.cmd_list.CopyBufferRegion(
                    dst_buffer.resource.deref(),
                    region.dst_offset_in_bytes,
                    src_buffer.resource.deref(),
                    region.src_offset_in_bytes,
                    region.size_in_bytes,
                );
            }
        }
    }

    fn cmd_copy_buffer_to_texture_regions(
        &self,
        cmd_list: &mut CommandList,
        src_buffer: &Buffer,
        dst_texture: &Texture,
        regions: &[BufferToTextureCopyRegion],
    ) {
        let cmd_list = unsafe {
            cmd_list
                .backend_data
                .downcast_ref::<D3D12CommandList>()
                .unwrap_unchecked()
        };

        let d3d_src_buffer = unsafe {
            src_buffer
                .backend_data
                .downcast_ref::<D3D12Buffer>()
                .unwrap_unchecked()
        };

        let d3d_dst_texture = unsafe {
            dst_texture
                .backend_data
                .downcast_ref::<D3D12Texture>()
                .unwrap_unchecked()
        };

        for region in regions {
            let src_location = D3D12_TEXTURE_COPY_LOCATION {
                pResource: Some(d3d_src_buffer.resource.deref().clone()),
                Type: D3D12_TEXTURE_COPY_TYPE_PLACED_FOOTPRINT,
                Anonymous: D3D12_TEXTURE_COPY_LOCATION_0 {
                    PlacedFootprint: D3D12_PLACED_SUBRESOURCE_FOOTPRINT {
                        Offset: region.buffer_offset_in_bytes,
                        Footprint: D3D12_SUBRESOURCE_FOOTPRINT {
                            Format: get_dxgi_format_from_ze_format(dst_texture.desc.format),
                            Width: region.buffer_texture_width as u32,
                            Height: region.buffer_texture_height as u32,
                            Depth: region.buffer_texture_depth as u32,
                            RowPitch: region.buffer_texture_row_pitch_in_bytes as u32,
                        },
                    },
                },
            };

            let dst_location = D3D12_TEXTURE_COPY_LOCATION {
                pResource: Some(d3d_dst_texture.texture.deref().clone()),
                Type: D3D12_TEXTURE_COPY_TYPE_SUBRESOURCE_INDEX,
                Anonymous: D3D12_TEXTURE_COPY_LOCATION_0 {
                    SubresourceIndex: region.texture_subresource_index,
                },
            };

            unsafe {
                cmd_list.cmd_list.CopyTextureRegion(
                    &dst_location,
                    region.texture_subresource_offset.x as u32,
                    region.texture_subresource_offset.y as u32,
                    region.texture_subresource_offset.z as u32,
                    &src_location,
                    None,
                )
            };
        }
    }

    #[cfg(feature = "pix")]
    fn cmd_debug_begin_event(&self, cmd_list: &mut CommandList, name: &str, color: Color4f32) {
        use ze_core::color::Color4u8;

        let cmd_list = unsafe {
            cmd_list
                .backend_data
                .downcast_ref::<D3D12CommandList>()
                .unwrap_unchecked()
        };

        let mut c_name: Vec<u16> = name.encode_utf16().collect();
        c_name.push(0);
        let color: Color4u8 = color.into();
        unsafe {
            let cmd_list = mem::transmute_copy::<
                ID3D12GraphicsCommandList4,
                *mut pix::ID3D12GraphicsCommandList,
            >(&cmd_list.cmd_list.0);

            pix_begin_event_cmd_list(cmd_list, color.r, color.g, color.b, c_name.as_ptr());
        }
    }

    #[cfg(not(feature = "pix"))]
    fn cmd_debug_begin_event(&self, _: &mut CommandList, _: &str, _: Color4f32) {}

    #[cfg(feature = "pix")]
    fn cmd_debug_end_event(&self, cmd_list: &mut CommandList) {
        let cmd_list = unsafe {
            cmd_list
                .backend_data
                .downcast_ref::<D3D12CommandList>()
                .unwrap_unchecked()
        };

        unsafe {
            let cmd_list = mem::transmute_copy::<
                ID3D12GraphicsCommandList4,
                *mut pix::ID3D12GraphicsCommandList,
            >(&cmd_list.cmd_list.0);

            pix_end_event_cmd_list(cmd_list);
        }
    }

    #[cfg(not(feature = "pix"))]
    fn cmd_debug_end_event(&self, _: &mut CommandList) {}

    fn cmd_begin_render_pass(&self, cmd_list: &mut CommandList, desc: &RenderPassDesc) {
        let mut cmd_list = unsafe {
            cmd_list
                .backend_data
                .downcast_mut::<D3D12CommandList>()
                .unwrap_unchecked()
        };

        let mut render_target_descs = vec![];

        cmd_list.render_pass_rt_count = desc.render_targets.len() as u32;
        for (i, desc) in desc.render_targets.iter().enumerate() {
            let rtv = unsafe {
                desc.render_target_view
                    .backend_data
                    .downcast_ref::<D3D12RenderTargetView>()
                    .unwrap_unchecked()
            };

            let clear_color = match desc.clear_value {
                ClearValue::Color(color) => D3D12_CLEAR_VALUE_0 { Color: color },
                ClearValue::DepthStencil((depth, stencil)) => D3D12_CLEAR_VALUE_0 {
                    DepthStencil: D3D12_DEPTH_STENCIL_VALUE {
                        Depth: depth,
                        Stencil: stencil,
                    },
                },
            };

            debug_assert!(
                desc.store_mode != RenderPassTextureStoreMode::Resolve,
                "Non-implemented"
            );

            cmd_list.render_pass_rtv_formats[i] =
                get_dxgi_format_from_ze_format(desc.render_target_view.desc.format);

            render_target_descs.push(D3D12_RENDER_PASS_RENDER_TARGET_DESC {
                cpuDescriptor: rtv.handle.0,
                BeginningAccess: D3D12_RENDER_PASS_BEGINNING_ACCESS {
                    Type: get_d3d_render_pass_beginning_access_type_from_ze_load_mode(
                        desc.load_mode,
                    ),
                    Anonymous: D3D12_RENDER_PASS_BEGINNING_ACCESS_0 {
                        Clear: D3D12_RENDER_PASS_BEGINNING_ACCESS_CLEAR_PARAMETERS {
                            ClearValue: D3D12_CLEAR_VALUE {
                                Format: get_dxgi_format_from_ze_format(
                                    desc.render_target_view.desc.format,
                                ),
                                Anonymous: clear_color,
                            },
                        },
                    },
                },
                EndingAccess: D3D12_RENDER_PASS_ENDING_ACCESS {
                    Type: get_d3d_render_pass_ending_access_type_from_ze_store_mode(
                        desc.store_mode,
                    ),
                    Anonymous: Default::default(),
                },
            });
        }

        let mut depth_test = D3D12_RENDER_PASS_DEPTH_STENCIL_DESC::default();
        if let Some(depth_stencil_desc) = &desc.depth_stencil {
            let dsv = unsafe {
                depth_stencil_desc
                    .depth_stencil_view
                    .backend_data
                    .downcast_ref::<D3D12DepthStencilView>()
                    .unwrap_unchecked()
            };

            let clear_color = match depth_stencil_desc.clear_value {
                ClearValue::Color(color) => D3D12_CLEAR_VALUE_0 { Color: color },
                ClearValue::DepthStencil((depth, stencil)) => D3D12_CLEAR_VALUE_0 {
                    DepthStencil: D3D12_DEPTH_STENCIL_VALUE {
                        Depth: depth,
                        Stencil: stencil,
                    },
                },
            };

            depth_test = D3D12_RENDER_PASS_DEPTH_STENCIL_DESC {
                cpuDescriptor: dsv.handle.0,
                DepthBeginningAccess: D3D12_RENDER_PASS_BEGINNING_ACCESS {
                    Type: get_d3d_render_pass_beginning_access_type_from_ze_load_mode(
                        depth_stencil_desc.load_mode,
                    ),
                    Anonymous: D3D12_RENDER_PASS_BEGINNING_ACCESS_0 {
                        Clear: D3D12_RENDER_PASS_BEGINNING_ACCESS_CLEAR_PARAMETERS {
                            ClearValue: D3D12_CLEAR_VALUE {
                                Format: get_dxgi_format_from_ze_format(
                                    depth_stencil_desc.depth_stencil_view.desc.format,
                                ),
                                Anonymous: clear_color,
                            },
                        },
                    },
                },
                StencilBeginningAccess: Default::default(),
                DepthEndingAccess: D3D12_RENDER_PASS_ENDING_ACCESS {
                    Type: get_d3d_render_pass_ending_access_type_from_ze_store_mode(
                        depth_stencil_desc.store_mode,
                    ),
                    Anonymous: Default::default(),
                },
                StencilEndingAccess: Default::default(),
            }
        };

        unsafe {
            cmd_list.cmd_list.BeginRenderPass(
                Some(&render_target_descs),
                if desc.depth_stencil.is_some() {
                    Some(&depth_test)
                } else {
                    None
                },
                D3D12_RENDER_PASS_FLAG_NONE,
            );
        }

        cmd_list.pipeline_state_dirty = true;
    }

    fn cmd_end_render_pass(&self, cmd_list: &mut CommandList) {
        let cmd_list = unsafe {
            cmd_list
                .backend_data
                .downcast_ref::<D3D12CommandList>()
                .unwrap_unchecked()
        };

        unsafe {
            cmd_list.cmd_list.EndRenderPass();
        }
    }

    fn cmd_resource_barrier(&self, cmd_list: &mut CommandList, barriers: &[ResourceBarrier]) {
        let cmd_list = unsafe {
            cmd_list
                .backend_data
                .downcast_ref::<D3D12CommandList>()
                .unwrap_unchecked()
        };

        let mut resource_barriers = Vec::with_capacity(barriers.len());
        for barrier in barriers {
            match barrier {
                ResourceBarrier::Transition(transition) => {
                    let resource = match transition.resource {
                        ResourceTransitionBarrierResource::Buffer(buffer) => buffer
                            .backend_data
                            .downcast_ref::<D3D12Buffer>()
                            .unwrap()
                            .resource
                            .deref(),
                        ResourceTransitionBarrierResource::Texture(texture) => texture
                            .backend_data
                            .downcast_ref::<D3D12Texture>()
                            .unwrap()
                            .texture
                            .deref(),
                    };

                    resource_barriers.push(D3D12_RESOURCE_BARRIER {
                        Type: D3D12_RESOURCE_BARRIER_TYPE_TRANSITION,
                        Flags: D3D12_RESOURCE_BARRIER_FLAG_NONE,
                        Anonymous: D3D12_RESOURCE_BARRIER_0 {
                            Transition: ManuallyDrop::new(D3D12_RESOURCE_TRANSITION_BARRIER {
                                pResource: Some(resource.clone()),
                                Subresource: D3D12_RESOURCE_BARRIER_ALL_SUBRESOURCES,
                                StateBefore: get_d3d_resource_stats_from_ze_resource_state(
                                    transition.source_state,
                                ),
                                StateAfter: get_d3d_resource_stats_from_ze_resource_state(
                                    transition.dest_state,
                                ),
                            }),
                        },
                    });
                }
            }
        }

        unsafe {
            cmd_list.cmd_list.ResourceBarrier(&resource_barriers);
        }

        // We need to call drops or else we're going to leak COM objects
        for barrier in resource_barriers {
            match barrier.Type {
                D3D12_RESOURCE_BARRIER_TYPE_TRANSITION => {
                    let transition_barrier = unsafe { barrier.Anonymous.Transition };
                    drop(ManuallyDrop::into_inner(transition_barrier));
                }
                _ => todo!(),
            }
        }
    }

    fn cmd_set_viewports(&self, cmd_list: &mut CommandList, viewports: &[Viewport]) {
        let cmd_list = unsafe {
            cmd_list
                .backend_data
                .downcast_mut::<D3D12CommandList>()
                .unwrap_unchecked()
        };

        let mut d3d_viewports = Vec::with_capacity(viewports.len());
        for viewport in viewports {
            d3d_viewports.push(D3D12_VIEWPORT {
                TopLeftX: viewport.position.x,
                TopLeftY: viewport.position.y,
                Width: viewport.size.x,
                Height: viewport.size.y,
                MinDepth: viewport.min_depth,
                MaxDepth: viewport.max_depth,
            });
        }

        unsafe {
            cmd_list.cmd_list.RSSetViewports(&d3d_viewports);
        }
    }

    fn cmd_set_scissors(&self, cmd_list: &mut CommandList, scissors: &[RectI32]) {
        let cmd_list = unsafe {
            cmd_list
                .backend_data
                .downcast_mut::<D3D12CommandList>()
                .unwrap_unchecked()
        };

        let mut d3d_scissors = Vec::with_capacity(scissors.len());
        for scissor in scissors {
            d3d_scissors.push(RECT {
                left: scissor.x,
                top: scissor.y,
                right: scissor.width,
                bottom: scissor.height,
            });
        }

        unsafe {
            cmd_list.cmd_list.RSSetScissorRects(&d3d_scissors);
        }
    }

    fn cmd_set_shader_stages(&self, cmd_list: &mut CommandList, stages: &[PipelineShaderStage]) {
        let mut cmd_list = unsafe {
            cmd_list
                .backend_data
                .downcast_mut::<D3D12CommandList>()
                .unwrap_unchecked()
        };

        cmd_list.pipeline_state_dirty = true;

        if stages.len() == 1 && stages[0].stage == ShaderStageFlagBits::Compute {
            let desc = D3D12_COMPUTE_PIPELINE_STATE_DESC::default();
            //desc.CS = D3D12_SHADER_BYTECODE {};
            cmd_list.pipeline = D3D12CommandListPipelineType::Compute(desc);
            todo!();
        } else {
            let desc = match &mut cmd_list.pipeline {
                D3D12CommandListPipelineType::Graphics(graphics) => graphics,
                _ => {
                    cmd_list.pipeline = D3D12CommandListPipelineType::Graphics(
                        GraphicsPipelineStateDesc::new((*self.default_root_signature).clone()),
                    );
                    cmd_list.pipeline.as_graphics_mut()
                }
            };

            for shader in stages {
                let module = unsafe {
                    shader
                        .module
                        .backend_data
                        .downcast_ref::<D3D12ShaderModule>()
                        .unwrap_unchecked()
                };

                let bytecode = D3D12_SHADER_BYTECODE {
                    pShaderBytecode: module.bytecode.as_ptr() as *const c_void,
                    BytecodeLength: module.bytecode.len(),
                };

                match shader.stage {
                    ShaderStageFlagBits::Vertex => {
                        *desc.vertex_shader = bytecode;
                        *desc.mesh_shader = Default::default();
                    }
                    ShaderStageFlagBits::Fragment => *desc.pixel_shader = bytecode,
                    ShaderStageFlagBits::Mesh => {
                        *desc.mesh_shader = bytecode;
                        *desc.vertex_shader = Default::default();
                    }
                    ShaderStageFlagBits::Compute => {
                        panic!("Cannot have a compute stage in a graphics pipeline!")
                    }
                }
            }
        }
    }

    fn cmd_set_input_assembly_state(
        &self,
        cmd_list: &mut CommandList,
        state: &PipelineInputAssemblyState,
    ) {
        let mut cmd_list = unsafe {
            cmd_list
                .backend_data
                .downcast_mut::<D3D12CommandList>()
                .unwrap_unchecked()
        };

        if let D3D12CommandListPipelineType::Graphics(graphics) = &mut cmd_list.pipeline {
            *graphics.primitive_topology_type = match state.primitive_topology {
                PrimitiveTopology::Point => D3D12_PRIMITIVE_TOPOLOGY_TYPE_POINT,
                PrimitiveTopology::Line => D3D12_PRIMITIVE_TOPOLOGY_TYPE_LINE,
                PrimitiveTopology::Triangle => D3D12_PRIMITIVE_TOPOLOGY_TYPE_TRIANGLE,
            }
        }

        cmd_list.pipeline_state_dirty = true;
    }

    fn cmd_set_blend_state(&self, cmd_list: &mut CommandList, state: &PipelineBlendState) {
        let mut cmd_list = unsafe {
            cmd_list
                .backend_data
                .downcast_mut::<D3D12CommandList>()
                .unwrap_unchecked()
        };

        if let D3D12CommandListPipelineType::Graphics(graphics) = &mut cmd_list.pipeline {
            let convert_blend = |blend: BlendFactor| -> D3D12_BLEND {
                match blend {
                    BlendFactor::Zero => D3D12_BLEND_ZERO,
                    BlendFactor::One => D3D12_BLEND_ONE,
                    BlendFactor::SrcColor => D3D12_BLEND_SRC_COLOR,
                    BlendFactor::OneMinusSrcColor => D3D12_BLEND_INV_SRC_COLOR,
                    BlendFactor::DstColor => D3D12_BLEND_DEST_COLOR,
                    BlendFactor::OneMinusDstColor => D3D12_BLEND_INV_DEST_COLOR,
                    BlendFactor::SrcAlpha => D3D12_BLEND_SRC_ALPHA,
                    BlendFactor::OneMinusSrcAlpha => D3D12_BLEND_INV_SRC_ALPHA,
                    BlendFactor::DstAlpha => D3D12_BLEND_DEST_ALPHA,
                    BlendFactor::OneMinusDstAlpha => D3D12_BLEND_INV_DEST_ALPHA,
                }
            };

            let convert_blend_op = |blend_op: BlendOp| -> D3D12_BLEND_OP {
                match blend_op {
                    BlendOp::Add => D3D12_BLEND_OP_ADD,
                    BlendOp::Subtract => D3D12_BLEND_OP_SUBTRACT,
                    BlendOp::ReverseSubtract => D3D12_BLEND_OP_REV_SUBTRACT,
                    BlendOp::Min => D3D12_BLEND_OP_MIN,
                    BlendOp::Max => D3D12_BLEND_OP_MAX,
                }
            };

            let mut render_targets = [D3D12_RENDER_TARGET_BLEND_DESC::default(); 8];
            for (i, render_target) in state.render_targets.iter().enumerate() {
                render_targets[i].BlendEnable = BOOL::from(render_target.enable_blend);
                render_targets[i].SrcBlend = convert_blend(render_target.src_color_blend_factor);
                render_targets[i].DestBlend = convert_blend(render_target.dst_color_blend_factor);
                render_targets[i].BlendOp = convert_blend_op(render_target.color_blend_op);
                render_targets[i].SrcBlendAlpha =
                    convert_blend(render_target.src_alpha_blend_factor);
                render_targets[i].DestBlendAlpha =
                    convert_blend(render_target.dst_alpha_blend_factor);
                render_targets[i].BlendOpAlpha = convert_blend_op(render_target.color_blend_op);
                render_targets[i].RenderTargetWriteMask = D3D12_COLOR_WRITE_ENABLE_ALL.0 as u8;
            }

            *graphics.blend_state = D3D12_BLEND_DESC {
                AlphaToCoverageEnable: BOOL::from(false),
                IndependentBlendEnable: BOOL::from(false),
                RenderTarget: render_targets,
            };

            cmd_list.pipeline_state_dirty = true;
        }
    }

    fn cmd_set_depth_stencil_state(
        &self,
        cmd_list: &mut CommandList,
        state: &PipelineDepthStencilState,
    ) {
        let mut cmd_list = unsafe {
            cmd_list
                .backend_data
                .downcast_mut::<D3D12CommandList>()
                .unwrap_unchecked()
        };

        if let D3D12CommandListPipelineType::Graphics(graphics) = &mut cmd_list.pipeline {
            *graphics.depth_stencil_state = D3D12_DEPTH_STENCIL_DESC {
                DepthEnable: BOOL::from(state.depth_test_enable),
                DepthWriteMask: D3D12_DEPTH_WRITE_MASK(state.depth_write_mask),
                DepthFunc: get_d3d_compare_func_from_ze_compare_op(state.depth_compare_op),
                StencilEnable: BOOL::from(state.stencil_test_enable),
                StencilReadMask: state.stencil_read_mask,
                StencilWriteMask: state.stencil_write_mask,
                FrontFace: get_d3d_depth_stencil_op_desc(&state.front),
                BackFace: get_d3d_depth_stencil_op_desc(&state.back),
            };
            cmd_list.pipeline_state_dirty = true;
        }
    }

    fn cmd_bind_index_buffer(
        &self,
        cmd_list: &mut CommandList,
        index_buffer: &Buffer,
        format: IndexBufferFormat,
    ) {
        let cmd_list = unsafe {
            cmd_list
                .backend_data
                .downcast_mut::<D3D12CommandList>()
                .unwrap_unchecked()
        };

        let buffer = unsafe {
            index_buffer
                .backend_data
                .downcast_ref::<D3D12Buffer>()
                .unwrap_unchecked()
        };

        unsafe {
            let view = D3D12_INDEX_BUFFER_VIEW {
                BufferLocation: buffer.gpu_virtual_address,
                SizeInBytes: index_buffer.info.size_bytes as u32,
                Format: match format {
                    IndexBufferFormat::Uint16 => DXGI_FORMAT_R16_UINT,
                    IndexBufferFormat::Uint32 => DXGI_FORMAT_R32_UINT,
                },
            };
            cmd_list.cmd_list.IASetIndexBuffer(Some(&view));
        }
    }

    fn cmd_push_constants(&self, cmd_list: &mut CommandList, offset_in_bytes: u32, data: &[u8]) {
        let cmd_list = unsafe {
            cmd_list
                .backend_data
                .downcast_mut::<D3D12CommandList>()
                .unwrap_unchecked()
        };

        unsafe {
            cmd_list.cmd_list.SetGraphicsRoot32BitConstants(
                0,
                (data.len() / size_of::<u32>()) as u32,
                data.as_ptr() as *const c_void,
                offset_in_bytes / size_of::<u32>() as u32,
            );
        }
    }

    fn cmd_draw(
        &self,
        cmd_list: &mut CommandList,
        vertex_count_per_instance: u32,
        instance_count: u32,
        first_vertex: u32,
        first_instance: u32,
    ) {
        let cmd_list = unsafe {
            cmd_list
                .backend_data
                .downcast_mut::<D3D12CommandList>()
                .unwrap_unchecked()
        };

        self.flush_pipeline_state(cmd_list);
        unsafe {
            cmd_list.cmd_list.DrawInstanced(
                vertex_count_per_instance,
                instance_count,
                first_vertex,
                first_instance,
            )
        };
    }

    fn cmd_draw_indexed(
        &self,
        cmd_list: &mut CommandList,
        index_count_per_instance: u32,
        instance_count: u32,
        first_index: u32,
        first_instance: u32,
    ) {
        let cmd_list = unsafe {
            cmd_list
                .backend_data
                .downcast_mut::<D3D12CommandList>()
                .unwrap_unchecked()
        };

        self.flush_pipeline_state(cmd_list);
        unsafe {
            cmd_list.cmd_list.DrawIndexedInstanced(
                index_count_per_instance,
                instance_count,
                first_index,
                0, // Unused as we don't use any vertex buffers
                first_instance,
            )
        };
    }

    fn cmd_dispatch_mesh(
        &self,
        cmd_list: &mut CommandList,
        thread_group_x: u32,
        thread_group_y: u32,
        thread_group_z: u32,
    ) {
        let cmd_list = unsafe {
            cmd_list
                .backend_data
                .downcast_mut::<D3D12CommandList>()
                .unwrap_unchecked()
        };

        self.flush_pipeline_state(cmd_list);
        unsafe {
            cmd_list
                .cmd_list
                .DispatchMesh(thread_group_x, thread_group_y, thread_group_z);
        };
    }

    fn submit(
        &self,
        queue_type: QueueType,
        command_lists: &[&CommandList],
        wait_fences: &[&Fence],
        signal_fences: &[&Fence],
    ) {
        self.frame_manager.current_frame().command_manager().submit(
            queue_type,
            command_lists,
            wait_fences,
            signal_fences,
        );
    }

    fn wait_idle(&self) {
        self.frame_manager.wait_for_work();
    }
}
