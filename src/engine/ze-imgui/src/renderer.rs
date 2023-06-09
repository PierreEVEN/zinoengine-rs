﻿use std::mem::{size_of, MaybeUninit};
use std::slice;
use std::sync::Arc;
use ze_gfx::backend::*;
use ze_imgui_sys::{ImDrawData, ImDrawIdx, ImDrawVert};

#[derive(Default)]
pub enum SwapChainType {
    Owned((MaybeUninit<Arc<SwapChain>>, Vec<RenderTargetView>)),

    #[default]
    Unowned,
}

impl Drop for SwapChainType {
    fn drop(&mut self) {
        match self {
            SwapChainType::Owned((swapchain, _)) => unsafe {
                swapchain.assume_init_drop();
            },
            SwapChainType::Unowned => {}
        }
    }
}

#[derive(Default)]
pub struct ViewportRendererData {
    pub swapchain: SwapChainType,
    pub vertex_buffer: Option<Arc<Buffer>>,
    pub vertex_buffer_srv: Option<ShaderResourceView>,
    pub index_buffer: Option<Arc<Buffer>>,
}

impl ViewportRendererData {
    pub fn update_buffers(&mut self, device: &Arc<dyn Device>, draw_data: &ImDrawData) {
        let vertex_buffer_size =
            (draw_data.TotalVtxCount as u64) * (size_of::<ImDrawVert>() as u64);
        let index_buffer_size = (draw_data.TotalIdxCount as u64) * (size_of::<ImDrawIdx>() as u64);

        if Self::create_or_resize_buffer(device, &mut self.vertex_buffer, vertex_buffer_size) {
            let srv = device
                .create_shader_resource_view(&ShaderResourceViewDesc::Buffer(BufferSRV {
                    buffer: self.vertex_buffer.as_ref().unwrap().clone(),
                    ty: BufferSRVType::Raw(BufferSRVRaw::default()),
                }))
                .expect("Failed to create ImGui vertex buffer srv");
            self.vertex_buffer_srv = Some(srv);
        }

        Self::create_or_resize_buffer(device, &mut self.index_buffer, index_buffer_size);

        if let (Some(vertex_buffer), Some(index_buffer)) = (&self.vertex_buffer, &self.index_buffer)
        {
            let mut vertex_ptr =
                device.buffer_mapped_ptr(vertex_buffer).unwrap() as *mut ImDrawVert;
            let mut index_ptr = device.buffer_mapped_ptr(index_buffer).unwrap() as *mut ImDrawIdx;

            let draw_lists = unsafe {
                slice::from_raw_parts(draw_data.CmdLists, draw_data.CmdListsCount as usize)
            };

            for draw_list in draw_lists {
                let draw_list = unsafe { draw_list.as_ref().unwrap_unchecked() };
                unsafe {
                    let vertex_buffer_slice = slice::from_raw_parts(
                        draw_list.VtxBuffer.Data,
                        draw_list.VtxBuffer.Size as usize,
                    );

                    let dst_vertex_slice =
                        slice::from_raw_parts_mut(vertex_ptr, draw_list.VtxBuffer.Size as usize);

                    dst_vertex_slice.copy_from_slice(vertex_buffer_slice);

                    let index_buffer_slice = slice::from_raw_parts(
                        draw_list.IdxBuffer.Data,
                        draw_list.IdxBuffer.Size as usize,
                    );

                    let dst_index_slice =
                        slice::from_raw_parts_mut(index_ptr, draw_list.IdxBuffer.Size as usize);

                    dst_index_slice.copy_from_slice(index_buffer_slice);

                    vertex_ptr = vertex_ptr.add(draw_list.VtxBuffer.Size as usize);
                    index_ptr = index_ptr.add(draw_list.IdxBuffer.Size as usize);
                }
            }
        }
    }

    fn create_or_resize_buffer(
        device: &Arc<dyn Device>,
        buffer: &mut Option<Arc<Buffer>>,
        size_bytes: u64,
    ) -> bool {
        if size_bytes == 0 {
            return false;
        }

        if let Some(buffer) = buffer {
            if buffer.info.size_bytes >= size_bytes {
                return false;
            }
        }

        let new_buffer = device
            .create_buffer(
                &BufferDesc {
                    size_bytes,
                    usage: BufferUsageFlags::default(),
                    memory_desc: MemoryDesc {
                        memory_location: MemoryLocation::CpuToGpu,
                        memory_flags: Default::default(),
                    },
                    default_resource_state: ResourceState::Common,
                },
                None,
                "ImGui Viewport Buffer",
            )
            .expect("Failed to create ImGui viewport vertex buffer");

        *buffer = Some(Arc::new(new_buffer));
        true
    }
}
