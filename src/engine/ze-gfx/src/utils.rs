﻿use crate::backend::{
    Buffer, BufferCopyRegion, BufferDesc, BufferToTextureCopyRegion, BufferUsageFlags, Device,
    DeviceError, MemoryDesc, MemoryLocation, QueueType, ResourceBarrier, ResourceState,
    ResourceTransitionBarrier, ResourceTransitionBarrierResource, Texture,
};
use std::ptr;
use std::sync::Arc;
use ze_core::maths::Vector3;

/// Copy data over to a buffer (using a staging buffer if required)
/// The source buffer MUST be in the Common state
/// The destination resource state must be a state that is understood by transfer queues
pub fn copy_data_to_buffer(
    device: &Arc<dyn Device>,
    buffer: &Buffer,
    data: &[u8],
    dst_resource_state: ResourceState,
) -> Result<(), DeviceError> {
    assert!(!data.is_empty());
    debug_assert!(
        dst_resource_state == ResourceState::Common
            || dst_resource_state == ResourceState::CopyRead
            || dst_resource_state == ResourceState::CopyWrite
    );

    if buffer.info.memory_desc.memory_location != MemoryLocation::CpuToGpu {
        let staging = device.create_buffer(
            &BufferDesc {
                size_bytes: buffer.info.size_bytes,
                usage: BufferUsageFlags::default(),
                memory_desc: MemoryDesc {
                    memory_location: MemoryLocation::CpuToGpu,
                    memory_flags: Default::default(),
                },
                default_resource_state: ResourceState::CopyRead,
            },
            None,
            "copy_data_to_buffer Staging buffer",
        )?;
        copy_data_to_buffer(device, &staging, data, dst_resource_state)?;

        let mut list = device.create_command_list(QueueType::Transfer)?;
        device.cmd_copy_buffer_regions(
            &mut list,
            &staging,
            buffer,
            &[BufferCopyRegion {
                src_offset_in_bytes: 0,
                dst_offset_in_bytes: 0,
                size_in_bytes: buffer.info.size_bytes,
            }],
        );

        if dst_resource_state != ResourceState::Common {
            device.cmd_resource_barrier(
                &mut list,
                &[ResourceBarrier::Transition(ResourceTransitionBarrier {
                    resource: ResourceTransitionBarrierResource::Buffer(buffer),
                    source_state: ResourceState::CopyWrite,
                    dest_state: dst_resource_state,
                })],
            );
        }
        device.submit(QueueType::Transfer, &[&list], &[], &[]);
    } else {
        let buffer_data = device.buffer_mapped_ptr(buffer).unwrap();
        unsafe {
            ptr::copy_nonoverlapping(data.as_ptr(), buffer_data, data.len());
        }
    }

    Ok(())
}

/// Copy data over to a texture (using a staging buffer)
/// The source buffer MUST be in the Common state
/// The destination resource state must be a state that is understood by transfer queues
pub fn copy_data_to_texture(
    device: &Arc<dyn Device>,
    data: &[u8],
    src_width: u32,
    src_height: u32,
    src_row_pitch_in_bytes: usize,
    texture: &Texture,
    dst_resource_state: ResourceState,
) -> Result<(), DeviceError> {
    assert!(!data.is_empty());
    assert_eq!(texture.desc.mip_levels, 1);
    debug_assert!(
        dst_resource_state == ResourceState::Common
            || dst_resource_state == ResourceState::CopyRead
            || dst_resource_state == ResourceState::CopyWrite
    );

    let subresource_layout = device.texture_subresource_layout(texture, 0);
    let staging = device.create_buffer(
        &BufferDesc {
            size_bytes: subresource_layout.size_in_bytes,
            usage: BufferUsageFlags::default(),
            memory_desc: MemoryDesc {
                memory_location: MemoryLocation::CpuToGpu,
                memory_flags: Default::default(),
            },
            default_resource_state: ResourceState::CopyRead,
        },
        None,
        "copy_data_to_texture Staging buffer",
    )?;

    let buffer_data = device.buffer_mapped_ptr(&staging).unwrap();
    unsafe {
        let width = src_width as usize;
        let height = src_height as usize;
        let row_pitch = subresource_layout.row_pitch_in_bytes as usize;

        for y in 0..height {
            ptr::copy_nonoverlapping(
                data.as_ptr().add(y * width * src_row_pitch_in_bytes),
                buffer_data.add(y * row_pitch),
                width * src_row_pitch_in_bytes,
            );
        }
    }

    let mut cmd_list = device.create_command_list(QueueType::Transfer)?;
    device.cmd_copy_buffer_to_texture_regions(
        &mut cmd_list,
        &staging,
        texture,
        &[BufferToTextureCopyRegion {
            buffer_offset_in_bytes: 0,
            buffer_texture_width: src_width,
            buffer_texture_height: src_height,
            buffer_texture_depth: 1,
            buffer_texture_row_pitch_in_bytes: subresource_layout.row_pitch_in_bytes as u32,
            texture_subresource_index: 0,
            texture_subresource_layout: subresource_layout,
            texture_subresource_width: texture.desc.width,
            texture_subresource_height: texture.desc.height,
            texture_subresource_depth: texture.desc.depth,
            texture_subresource_offset: Vector3::<i32>::default(),
        }],
    );
    device.submit(QueueType::Transfer, &[&cmd_list], &[], &[]);

    Ok(())
}
