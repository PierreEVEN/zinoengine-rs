use crate::pipeline_manager::GraphicsPipelineStateDesc;
use crate::utils::SendableIUnknown;
use windows::Win32::Graphics::Direct3D12::*;
use windows::Win32::Graphics::Dxgi::Common::*;
use ze_gfx::backend::MAX_RENDER_PASS_RENDER_TARGET_COUNT;

pub(crate) enum D3D12CommandListPipelineType {
    None,
    Graphics(GraphicsPipelineStateDesc),
    Compute(D3D12_COMPUTE_PIPELINE_STATE_DESC),
}

unsafe impl Send for D3D12CommandListPipelineType {}

impl D3D12CommandListPipelineType {
    pub fn as_graphics_mut(&mut self) -> &mut GraphicsPipelineStateDesc {
        match self {
            D3D12CommandListPipelineType::Graphics(pipeline) => pipeline,
            _ => panic!("Invalid pipeline type"),
        }
    }
}

pub(crate) struct D3D12CommandList {
    pub cmd_list: SendableIUnknown<ID3D12GraphicsCommandList6>,
    pub pipeline: D3D12CommandListPipelineType,
    pub render_pass_rt_count: u32,
    pub render_pass_rtv_formats: [DXGI_FORMAT; MAX_RENDER_PASS_RENDER_TARGET_COUNT],
    pub render_pass_dsv_format: DXGI_FORMAT,
    pub pipeline_state_dirty: bool,
}

impl D3D12CommandList {
    pub fn new(cmd_list: SendableIUnknown<ID3D12GraphicsCommandList6>) -> Self {
        Self {
            cmd_list,
            pipeline: D3D12CommandListPipelineType::None,
            render_pass_rt_count: 0,
            render_pass_rtv_formats: [DXGI_FORMAT_UNKNOWN; MAX_RENDER_PASS_RENDER_TARGET_COUNT],
            render_pass_dsv_format: DXGI_FORMAT_UNKNOWN,
            pipeline_state_dirty: true,
        }
    }

    pub fn cmd_list(&self) -> &ID3D12GraphicsCommandList6 {
        &self.cmd_list
    }
}
