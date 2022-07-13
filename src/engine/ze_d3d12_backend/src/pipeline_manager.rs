use crate::utils::SendableIUnknown;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::mem::transmute;
use std::ops::Deref;
use windows::Win32::Graphics::Direct3D12::*;

pub struct GraphicsPipelineEntry(D3D12_GRAPHICS_PIPELINE_STATE_DESC);

impl PartialEq for GraphicsPipelineEntry {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl Eq for GraphicsPipelineEntry {}

unsafe impl Send for GraphicsPipelineEntry {}
unsafe impl Sync for GraphicsPipelineEntry {}

impl Hash for GraphicsPipelineEntry {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write_usize(self.0.VS.pShaderBytecode as usize);
        state.write_usize(self.0.VS.BytecodeLength);
        state.write_usize(self.0.PS.pShaderBytecode as usize);
        state.write_usize(self.0.PS.BytecodeLength);
        state.write_u32(self.0.NumRenderTargets);

        // Rasterizer
        unsafe {
            state.write_i32(transmute(self.0.RasterizerState.FillMode));
            state.write_i32(transmute(self.0.RasterizerState.CullMode));
            state.write_i32(transmute(self.0.RasterizerState.FrontCounterClockwise));
            state.write_i32(self.0.RasterizerState.DepthBias);

            debug_assert_eq!(
                self.0.RasterizerState.DepthBiasClamp, 0.0,
                "Hash non-implemented yet"
            );

            debug_assert_eq!(
                self.0.RasterizerState.SlopeScaledDepthBias, 0.0,
                "Hash non-implemented yet"
            );

            state.write_i32(transmute(self.0.RasterizerState.DepthClipEnable));
            state.write_i32(transmute(self.0.RasterizerState.MultisampleEnable));
            state.write_i32(transmute(self.0.RasterizerState.AntialiasedLineEnable));
            state.write_u32(self.0.RasterizerState.ForcedSampleCount);
            state.write_i32(transmute(self.0.RasterizerState.ConservativeRaster));
        }
        // Depth & stencil
        unsafe {
            state.write_i32(transmute(self.0.DepthStencilState.DepthEnable));
            state.write_i32(transmute(self.0.DepthStencilState.DepthWriteMask));
            state.write_i32(transmute(self.0.DepthStencilState.DepthFunc));
            state.write_i32(transmute(self.0.DepthStencilState.StencilEnable));
            state.write_u8(self.0.DepthStencilState.StencilReadMask);
            state.write_u8(self.0.DepthStencilState.StencilWriteMask);

            state.write_i32(transmute(self.0.DepthStencilState.FrontFace.StencilFailOp));
            state.write_i32(transmute(
                self.0.DepthStencilState.FrontFace.StencilDepthFailOp,
            ));
            state.write_i32(transmute(self.0.DepthStencilState.FrontFace.StencilPassOp));
            state.write_i32(transmute(self.0.DepthStencilState.FrontFace.StencilFunc));

            state.write_i32(transmute(self.0.DepthStencilState.BackFace.StencilFailOp));
            state.write_i32(transmute(
                self.0.DepthStencilState.BackFace.StencilDepthFailOp,
            ));
            state.write_i32(transmute(self.0.DepthStencilState.BackFace.StencilPassOp));
            state.write_i32(transmute(self.0.DepthStencilState.BackFace.StencilFunc));
        }

        unsafe {
            state.write_i32(transmute(self.0.PrimitiveTopologyType));
        }

        for format in self.0.RTVFormats {
            unsafe {
                state.write_u32(transmute(format));
            }
        }

        unsafe {
            state.write_u32(transmute(self.0.DSVFormat));
        }
    }
}

impl From<&D3D12_GRAPHICS_PIPELINE_STATE_DESC> for GraphicsPipelineEntry {
    fn from(desc: &D3D12_GRAPHICS_PIPELINE_STATE_DESC) -> Self {
        Self { 0: desc.clone() }
    }
}

#[derive(Default)]
pub struct PipelineManager {
    graphics_pipelines:
        RwLock<HashMap<GraphicsPipelineEntry, SendableIUnknown<ID3D12PipelineState>>>,
}

impl PipelineManager {
    pub fn get_graphics_pipeline(
        &self,
        device: &ID3D12Device,
        desc: &D3D12_GRAPHICS_PIPELINE_STATE_DESC,
    ) -> ID3D12PipelineState {
        let graphics_pipelines = self.graphics_pipelines.read();
        let entry = desc.into();
        if let Some(pipeline) = graphics_pipelines.get(&entry) {
            pipeline.deref().clone()
        } else {
            drop(graphics_pipelines);
            let mut graphics_pipelines = self.graphics_pipelines.write();
            let pipeline: ID3D12PipelineState =
                unsafe { device.CreateGraphicsPipelineState(desc) }.unwrap();
            graphics_pipelines.insert(entry, pipeline.clone().into());
            pipeline
        }
    }
}
