use std::ops::Deref;
use windows::core::*;
use windows::Win32;
use windows::Win32::Graphics::Direct3D12::*;
use windows::Win32::Graphics::Dxgi::Common::*;
use ze_gfx::backend::{
    CompareOp, DeviceError, Filter, MemoryLocation, PipelineStencilOpState,
    RenderPassTextureLoadMode, RenderPassTextureStoreMode, ResourceState, StencilOp,
    TextureAddressMode,
};
use ze_gfx::{PixelFormat, SampleDesc};

/// Struct used to wrap a IUnknown to become Send/Sync for uses with Mutexes and such
#[derive(Clone)]
pub struct SendableIUnknown<T: Interface>(pub T);

impl<T: Interface> SendableIUnknown<T> {
    pub fn new(object: T) -> Self {
        Self(object)
    }
}

impl<T: Interface> From<T> for SendableIUnknown<T> {
    fn from(object: T) -> Self {
        Self::new(object)
    }
}

unsafe impl<T: Interface> Send for SendableIUnknown<T> {}
unsafe impl<T: Interface> Sync for SendableIUnknown<T> {}

impl<T: Interface> Deref for SendableIUnknown<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

// Utils conversion functions
pub fn get_heap_type_from_memory_location(memory_location: MemoryLocation) -> D3D12_HEAP_TYPE {
    match memory_location {
        MemoryLocation::CpuToGpu => D3D12_HEAP_TYPE_UPLOAD,
        MemoryLocation::GpuOnly => D3D12_HEAP_TYPE_DEFAULT,
    }
}

pub fn get_dxgi_format_from_ze_format(format: PixelFormat) -> DXGI_FORMAT {
    match format {
        PixelFormat::Unknown => DXGI_FORMAT_UNKNOWN,
        PixelFormat::B8G8R8A8UnormSrgb => DXGI_FORMAT_B8G8R8A8_UNORM_SRGB,
        PixelFormat::B8G8R8A8Unorm => DXGI_FORMAT_B8G8R8A8_UNORM,
        PixelFormat::R8Unorm => DXGI_FORMAT_R8_UNORM,
        PixelFormat::R8G8B8A8Unorm => DXGI_FORMAT_R8G8B8A8_UNORM,
        PixelFormat::D24UnormS8Uint => DXGI_FORMAT_D24_UNORM_S8_UINT,
        PixelFormat::R16G16B16A16Sfloat => DXGI_FORMAT_R16G16B16A16_FLOAT,
        _ => todo!(),
    }
}

pub fn get_ze_format_from_dxgi_format(format: DXGI_FORMAT) -> PixelFormat {
    match format {
        DXGI_FORMAT_UNKNOWN => PixelFormat::Unknown,
        DXGI_FORMAT_B8G8R8A8_UNORM_SRGB => PixelFormat::B8G8R8A8UnormSrgb,
        DXGI_FORMAT_B8G8R8A8_UNORM => PixelFormat::B8G8R8A8Unorm,
        DXGI_FORMAT_R8_UNORM => PixelFormat::R8Unorm,
        DXGI_FORMAT_R8G8B8A8_UNORM => PixelFormat::R8G8B8A8Unorm,
        DXGI_FORMAT_D24_UNORM_S8_UINT => PixelFormat::D24UnormS8Uint,
        DXGI_FORMAT_R16G16B16A16_FLOAT => PixelFormat::R16G16B16A16Sfloat,
        _ => todo!(),
    }
}

pub fn get_dxgi_sample_desc_from_ze_sample_desc(sample_desc: SampleDesc) -> DXGI_SAMPLE_DESC {
    DXGI_SAMPLE_DESC {
        Count: sample_desc.count,
        Quality: sample_desc.quality,
    }
}

pub fn get_ze_sample_desc_from_dxgi_sample_desc(sample_desc: DXGI_SAMPLE_DESC) -> SampleDesc {
    SampleDesc {
        count: sample_desc.Count,
        quality: sample_desc.Quality,
    }
}

pub fn convert_d3d_error_to_ze_device_error(result: Error) -> DeviceError {
    match result.code() {
        Win32::Foundation::E_OUTOFMEMORY => DeviceError::OutOfMemory,
        Win32::Graphics::Dxgi::DXGI_ERROR_INVALID_CALL => DeviceError::InvalidParameters,
        _ => DeviceError::Unknown,
    }
}

pub fn get_d3d_render_pass_beginning_access_type_from_ze_load_mode(
    load: RenderPassTextureLoadMode,
) -> D3D12_RENDER_PASS_BEGINNING_ACCESS_TYPE {
    match load {
        RenderPassTextureLoadMode::Discard => D3D12_RENDER_PASS_BEGINNING_ACCESS_TYPE_DISCARD,
        RenderPassTextureLoadMode::Preserve => D3D12_RENDER_PASS_BEGINNING_ACCESS_TYPE_PRESERVE,
        RenderPassTextureLoadMode::Clear => D3D12_RENDER_PASS_BEGINNING_ACCESS_TYPE_CLEAR,
    }
}

pub fn get_d3d_render_pass_ending_access_type_from_ze_store_mode(
    store: RenderPassTextureStoreMode,
) -> D3D12_RENDER_PASS_ENDING_ACCESS_TYPE {
    match store {
        RenderPassTextureStoreMode::Discard => D3D12_RENDER_PASS_ENDING_ACCESS_TYPE_DISCARD,
        RenderPassTextureStoreMode::Preserve => D3D12_RENDER_PASS_ENDING_ACCESS_TYPE_PRESERVE,
        RenderPassTextureStoreMode::Resolve => D3D12_RENDER_PASS_ENDING_ACCESS_TYPE_RESOLVE,
    }
}

pub fn get_d3d_resource_stats_from_ze_resource_state(
    state: ResourceState,
) -> D3D12_RESOURCE_STATES {
    match state {
        ResourceState::Common => D3D12_RESOURCE_STATE_COMMON,
        ResourceState::IndexBufferRead => D3D12_RESOURCE_STATE_INDEX_BUFFER,
        ResourceState::UnorderedAccessReadWrite => D3D12_RESOURCE_STATE_UNORDERED_ACCESS,
        ResourceState::RenderTargetWrite => D3D12_RESOURCE_STATE_RENDER_TARGET,
        ResourceState::DepthRead => D3D12_RESOURCE_STATE_DEPTH_READ,
        ResourceState::DepthWrite => D3D12_RESOURCE_STATE_DEPTH_WRITE,
        ResourceState::ShaderRead => D3D12_RESOURCE_STATE_PIXEL_SHADER_RESOURCE,
        ResourceState::CopyRead => D3D12_RESOURCE_STATE_COPY_SOURCE,
        ResourceState::CopyWrite => D3D12_RESOURCE_STATE_COPY_DEST,
        ResourceState::Present => D3D12_RESOURCE_STATE_PRESENT,
    }
}

pub fn set_resource_name(resource: &ID3D12Object, str: &str) {
    unsafe {
        let mut name: Vec<u16> = str.encode_utf16().collect();
        name.push(0);
        resource.SetName(PCWSTR(name.as_ptr())).unwrap();
    }
}

pub fn get_d3d_filter_from_ze_filter(filter: Filter) -> D3D12_FILTER {
    match filter {
        Filter::Nearest => D3D12_FILTER_MIN_MAG_MIP_POINT,
        Filter::Linear => D3D12_FILTER_MIN_MAG_MIP_LINEAR,
    }
}

pub fn get_d3d_texture_address_mode_from_ze_texture_address_mode(
    address_mode: TextureAddressMode,
) -> D3D12_TEXTURE_ADDRESS_MODE {
    match address_mode {
        TextureAddressMode::Repeat => D3D12_TEXTURE_ADDRESS_MODE_WRAP,
        TextureAddressMode::Mirror => D3D12_TEXTURE_ADDRESS_MODE_MIRROR,
        TextureAddressMode::Clamp => D3D12_TEXTURE_ADDRESS_MODE_CLAMP,
    }
}

pub fn get_d3d_compare_func_from_ze_compare_op(op: CompareOp) -> D3D12_COMPARISON_FUNC {
    match op {
        CompareOp::Never => D3D12_COMPARISON_FUNC_NEVER,
        CompareOp::Less => D3D12_COMPARISON_FUNC_LESS,
        CompareOp::Equal => D3D12_COMPARISON_FUNC_EQUAL,
        CompareOp::LessEqual => D3D12_COMPARISON_FUNC_LESS_EQUAL,
        CompareOp::Greater => D3D12_COMPARISON_FUNC_GREATER,
        CompareOp::NotEqual => D3D12_COMPARISON_FUNC_NOT_EQUAL,
        CompareOp::GreaterEqual => D3D12_COMPARISON_FUNC_GREATER_EQUAL,
        CompareOp::Always => D3D12_COMPARISON_FUNC_ALWAYS,
    }
}

pub fn get_d3d_stencil_op(op: StencilOp) -> D3D12_STENCIL_OP {
    match op {
        StencilOp::Keep => D3D12_STENCIL_OP_KEEP,
        StencilOp::Zero => D3D12_STENCIL_OP_ZERO,
        StencilOp::Replace => D3D12_STENCIL_OP_REPLACE,
        StencilOp::IncrementAndClamp => D3D12_STENCIL_OP_INCR_SAT,
        StencilOp::DecrementAndClamp => D3D12_STENCIL_OP_DECR_SAT,
        StencilOp::Invert => D3D12_STENCIL_OP_INVERT,
        StencilOp::IncrementAndWrap => D3D12_STENCIL_OP_INCR,
        StencilOp::DecrementAndWrap => D3D12_STENCIL_OP_DECR,
    }
}

pub fn get_d3d_depth_stencil_op_desc(state: &PipelineStencilOpState) -> D3D12_DEPTH_STENCILOP_DESC {
    D3D12_DEPTH_STENCILOP_DESC {
        StencilFailOp: get_d3d_stencil_op(state.fail_op),
        StencilDepthFailOp: get_d3d_stencil_op(state.depth_fail_op),
        StencilPassOp: get_d3d_stencil_op(state.pass_op),
        StencilFunc: get_d3d_compare_func_from_ze_compare_op(state.compare_op),
    }
}
