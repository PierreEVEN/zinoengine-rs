#pragma once

#if ZE_BACKEND_D3D12
#define ZE_PUSH_CONSTANT
#else
#error "Backend doesn't support push constants"
#endif

typedef uint ResourceHandle;

// Bindless support
#if ZE_BACKEND_D3D12
inline Texture2D get_texture(ResourceHandle handle)
{
    return ResourceDescriptorHeap[NonUniformResourceIndex(handle)];
}

inline TextureCube get_texture_cube(ResourceHandle handle)
{
    return ResourceDescriptorHeap[NonUniformResourceIndex(handle)];
}

inline SamplerState get_sampler(ResourceHandle handle)
{
    return SamplerDescriptorHeap[NonUniformResourceIndex(handle)];
}

template<typename StructType>
inline StructuredBuffer<StructType> get_structured_buffer(ResourceHandle handle)
{
    return ResourceDescriptorHeap[NonUniformResourceIndex(handle)];
}

inline ByteAddressBuffer get_byte_address_buffer(ResourceHandle handle)
{
    return ResourceDescriptorHeap[NonUniformResourceIndex(handle)];
}

#else
#error "Backend doesn't support bindless"
#endif

float4 convert_u32_rgba_color_to_float4(uint val)
{
    float s = 1.0f / 255.0f;
    return float4(
        ((val >> 0) & 0xFF) * s,
        ((val >> 8) & 0xFF) * s,
        ((val >> 16) & 0xFF) * s,
        ((val >> 24) & 0xFF) * s);
}