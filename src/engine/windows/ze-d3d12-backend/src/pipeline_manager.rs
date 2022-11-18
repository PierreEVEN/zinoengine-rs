use crate::utils::SendableIUnknown;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::mem::{size_of_val, transmute};
use std::ops::{Deref, DerefMut};
use windows::Win32::Foundation::BOOL;
use windows::Win32::Graphics::Direct3D12::*;
use windows::Win32::Graphics::Dxgi::Common::*;
use ze_gfx::backend::MAX_RENDER_PASS_RENDER_TARGET_COUNT;

#[repr(C, align(8))]
#[derive(PartialEq, Eq, Copy, Clone)]
pub struct PipelineTypedField<T> {
    subobject_type: D3D12_PIPELINE_STATE_SUBOBJECT_TYPE,
    value: T,
}

impl<T> PipelineTypedField<T> {
    pub fn new(subobject_type: D3D12_PIPELINE_STATE_SUBOBJECT_TYPE, value: T) -> Self {
        Self {
            subobject_type,
            value,
        }
    }

    pub fn new_defaulted(subobject_type: D3D12_PIPELINE_STATE_SUBOBJECT_TYPE) -> Self
    where
        T: Default,
    {
        Self {
            subobject_type,
            value: T::default(),
        }
    }
}

impl<T> Deref for PipelineTypedField<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

impl<T> DerefMut for PipelineTypedField<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.value
    }
}

#[repr(C)]
#[derive(Clone, PartialEq, Eq)]
struct VertexGraphicsPipelineStateDescStream {
    pub root_signature: PipelineTypedField<ID3D12RootSignature>,
    pub vertex_shader: PipelineTypedField<D3D12_SHADER_BYTECODE>,
    pub pixel_shader: PipelineTypedField<D3D12_SHADER_BYTECODE>,
    pub stream_output: PipelineTypedField<D3D12_STREAM_OUTPUT_DESC>,
    pub blend_state: PipelineTypedField<D3D12_BLEND_DESC>,
    pub sample_mask: PipelineTypedField<u32>,
    pub rasterizer_state: PipelineTypedField<D3D12_RASTERIZER_DESC>,
    pub depth_stencil_state: PipelineTypedField<D3D12_DEPTH_STENCIL_DESC>,
    pub input_layout: PipelineTypedField<D3D12_INPUT_LAYOUT_DESC>,
    pub ib_strip_cut_value: PipelineTypedField<D3D12_INDEX_BUFFER_STRIP_CUT_VALUE>,
    pub primitive_topology_type: PipelineTypedField<D3D12_PRIMITIVE_TOPOLOGY_TYPE>,
    pub rtv_formats: PipelineTypedField<D3D12_RT_FORMAT_ARRAY>,
    pub dsv_format: PipelineTypedField<DXGI_FORMAT>,
    pub sample_desc: PipelineTypedField<DXGI_SAMPLE_DESC>,
    pub node_mask: PipelineTypedField<u32>,
    pub cached_pso: PipelineTypedField<D3D12_CACHED_PIPELINE_STATE>,
    pub flags: PipelineTypedField<D3D12_PIPELINE_STATE_FLAGS>,
}

#[repr(C)]
#[derive(Clone, PartialEq, Eq)]
struct MeshGraphicsPipelineStateDescStream {
    pub root_signature: PipelineTypedField<ID3D12RootSignature>,
    pub pixel_shader: PipelineTypedField<D3D12_SHADER_BYTECODE>,
    pub mesh_shader: PipelineTypedField<D3D12_SHADER_BYTECODE>,
    pub amplification_shader: PipelineTypedField<D3D12_SHADER_BYTECODE>,
    pub stream_output: PipelineTypedField<D3D12_STREAM_OUTPUT_DESC>,
    pub blend_state: PipelineTypedField<D3D12_BLEND_DESC>,
    pub sample_mask: PipelineTypedField<u32>,
    pub rasterizer_state: PipelineTypedField<D3D12_RASTERIZER_DESC>,
    pub depth_stencil_state: PipelineTypedField<D3D12_DEPTH_STENCIL_DESC>,
    pub input_layout: PipelineTypedField<D3D12_INPUT_LAYOUT_DESC>,
    pub ib_strip_cut_value: PipelineTypedField<D3D12_INDEX_BUFFER_STRIP_CUT_VALUE>,
    pub primitive_topology_type: PipelineTypedField<D3D12_PRIMITIVE_TOPOLOGY_TYPE>,
    pub rtv_formats: PipelineTypedField<D3D12_RT_FORMAT_ARRAY>,
    pub dsv_format: PipelineTypedField<DXGI_FORMAT>,
    pub sample_desc: PipelineTypedField<DXGI_SAMPLE_DESC>,
    pub node_mask: PipelineTypedField<u32>,
    pub cached_pso: PipelineTypedField<D3D12_CACHED_PIPELINE_STATE>,
    pub flags: PipelineTypedField<D3D12_PIPELINE_STATE_FLAGS>,
}

// TODO: rework
#[derive(Clone, PartialEq, Eq)]
pub struct GraphicsPipelineStateDesc {
    pub root_signature: PipelineTypedField<ID3D12RootSignature>,
    pub vertex_shader: PipelineTypedField<D3D12_SHADER_BYTECODE>,
    pub pixel_shader: PipelineTypedField<D3D12_SHADER_BYTECODE>,
    pub mesh_shader: PipelineTypedField<D3D12_SHADER_BYTECODE>,
    pub amplification_shader: PipelineTypedField<D3D12_SHADER_BYTECODE>,
    pub stream_output: PipelineTypedField<D3D12_STREAM_OUTPUT_DESC>,
    pub blend_state: PipelineTypedField<D3D12_BLEND_DESC>,
    pub sample_mask: PipelineTypedField<u32>,
    pub rasterizer_state: PipelineTypedField<D3D12_RASTERIZER_DESC>,
    pub depth_stencil_state: PipelineTypedField<D3D12_DEPTH_STENCIL_DESC>,
    pub input_layout: PipelineTypedField<D3D12_INPUT_LAYOUT_DESC>,
    pub ib_strip_cut_value: PipelineTypedField<D3D12_INDEX_BUFFER_STRIP_CUT_VALUE>,
    pub primitive_topology_type: PipelineTypedField<D3D12_PRIMITIVE_TOPOLOGY_TYPE>,
    pub rtv_formats: PipelineTypedField<D3D12_RT_FORMAT_ARRAY>,
    pub dsv_format: PipelineTypedField<DXGI_FORMAT>,
    pub sample_desc: PipelineTypedField<DXGI_SAMPLE_DESC>,
    pub node_mask: PipelineTypedField<u32>,
    pub cached_pso: PipelineTypedField<D3D12_CACHED_PIPELINE_STATE>,
    pub flags: PipelineTypedField<D3D12_PIPELINE_STATE_FLAGS>,
}

impl GraphicsPipelineStateDesc {
    pub fn new(root_signature: ID3D12RootSignature) -> Self {
        let default_blend_desc = D3D12_RENDER_TARGET_BLEND_DESC {
            BlendEnable: Default::default(),
            LogicOpEnable: Default::default(),
            SrcBlend: Default::default(),
            DestBlend: Default::default(),
            BlendOp: Default::default(),
            SrcBlendAlpha: Default::default(),
            DestBlendAlpha: Default::default(),
            BlendOpAlpha: Default::default(),
            LogicOp: Default::default(),
            RenderTargetWriteMask: D3D12_COLOR_WRITE_ENABLE_ALL.0 as u8,
        };

        Self {
            root_signature: PipelineTypedField::new(
                D3D12_PIPELINE_STATE_SUBOBJECT_TYPE_ROOT_SIGNATURE,
                root_signature,
            ),
            vertex_shader: PipelineTypedField::new_defaulted(
                D3D12_PIPELINE_STATE_SUBOBJECT_TYPE_VS,
            ),
            pixel_shader: PipelineTypedField::new_defaulted(D3D12_PIPELINE_STATE_SUBOBJECT_TYPE_PS),
            mesh_shader: PipelineTypedField::new_defaulted(D3D12_PIPELINE_STATE_SUBOBJECT_TYPE_MS),
            amplification_shader: PipelineTypedField::new_defaulted(
                D3D12_PIPELINE_STATE_SUBOBJECT_TYPE_AS,
            ),
            stream_output: PipelineTypedField::new_defaulted(
                D3D12_PIPELINE_STATE_SUBOBJECT_TYPE_STREAM_OUTPUT,
            ),
            blend_state: PipelineTypedField::new(
                D3D12_PIPELINE_STATE_SUBOBJECT_TYPE_BLEND,
                D3D12_BLEND_DESC {
                    AlphaToCoverageEnable: Default::default(),
                    IndependentBlendEnable: Default::default(),
                    RenderTarget: [default_blend_desc; MAX_RENDER_PASS_RENDER_TARGET_COUNT],
                },
            ),
            sample_mask: PipelineTypedField::new(
                D3D12_PIPELINE_STATE_SUBOBJECT_TYPE_SAMPLE_MASK,
                u32::MAX,
            ),
            rasterizer_state: PipelineTypedField::new(
                D3D12_PIPELINE_STATE_SUBOBJECT_TYPE_RASTERIZER,
                D3D12_RASTERIZER_DESC {
                    FillMode: D3D12_FILL_MODE_SOLID,
                    CullMode: D3D12_CULL_MODE_NONE,
                    FrontCounterClockwise: BOOL::from(true),
                    DepthBias: 0,
                    DepthBiasClamp: 0.0,
                    SlopeScaledDepthBias: 0.0,
                    DepthClipEnable: Default::default(),
                    MultisampleEnable: Default::default(),
                    AntialiasedLineEnable: Default::default(),
                    ForcedSampleCount: 0,
                    ConservativeRaster: Default::default(),
                },
            ),
            depth_stencil_state: PipelineTypedField::new_defaulted(
                D3D12_PIPELINE_STATE_SUBOBJECT_TYPE_DEPTH_STENCIL,
            ),
            input_layout: PipelineTypedField::new_defaulted(
                D3D12_PIPELINE_STATE_SUBOBJECT_TYPE_INPUT_LAYOUT,
            ),
            ib_strip_cut_value: PipelineTypedField::new_defaulted(
                D3D12_PIPELINE_STATE_SUBOBJECT_TYPE_IB_STRIP_CUT_VALUE,
            ),
            primitive_topology_type: PipelineTypedField::new(
                D3D12_PIPELINE_STATE_SUBOBJECT_TYPE_PRIMITIVE_TOPOLOGY,
                D3D12_PRIMITIVE_TOPOLOGY_TYPE_TRIANGLE,
            ),
            rtv_formats: PipelineTypedField::new_defaulted(
                D3D12_PIPELINE_STATE_SUBOBJECT_TYPE_RENDER_TARGET_FORMATS,
            ),
            dsv_format: PipelineTypedField::new_defaulted(
                D3D12_PIPELINE_STATE_SUBOBJECT_TYPE_DEPTH_STENCIL_FORMAT,
            ),
            sample_desc: PipelineTypedField::new(
                D3D12_PIPELINE_STATE_SUBOBJECT_TYPE_SAMPLE_DESC,
                DXGI_SAMPLE_DESC {
                    Count: 1,
                    Quality: 0,
                },
            ),
            node_mask: PipelineTypedField::new_defaulted(
                D3D12_PIPELINE_STATE_SUBOBJECT_TYPE_NODE_MASK,
            ),
            cached_pso: PipelineTypedField::new_defaulted(
                D3D12_PIPELINE_STATE_SUBOBJECT_TYPE_CACHED_PSO,
            ),
            flags: PipelineTypedField::new_defaulted(D3D12_PIPELINE_STATE_SUBOBJECT_TYPE_FLAGS),
        }
    }
}

pub struct GraphicsPipelineEntry(GraphicsPipelineStateDesc);

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
        state.write_usize(self.0.vertex_shader.pShaderBytecode as usize);
        state.write_usize(self.0.vertex_shader.BytecodeLength);
        state.write_usize(self.0.pixel_shader.pShaderBytecode as usize);
        state.write_usize(self.0.pixel_shader.BytecodeLength);
        state.write_usize(self.0.mesh_shader.pShaderBytecode as usize);
        state.write_usize(self.0.mesh_shader.BytecodeLength);
        state.write_usize(self.0.amplification_shader.pShaderBytecode as usize);
        state.write_usize(self.0.amplification_shader.BytecodeLength);

        // Rasterizer
        unsafe {
            state.write_i32(transmute(self.0.rasterizer_state.FillMode));
            state.write_i32(transmute(self.0.rasterizer_state.CullMode));
            state.write_i32(transmute(self.0.rasterizer_state.FrontCounterClockwise));
            state.write_i32(self.0.rasterizer_state.DepthBias);

            debug_assert_eq!(
                self.0.rasterizer_state.DepthBiasClamp, 0.0,
                "Hash non-implemented yet"
            );

            debug_assert_eq!(
                self.0.rasterizer_state.SlopeScaledDepthBias, 0.0,
                "Hash non-implemented yet"
            );

            state.write_i32(transmute(self.0.rasterizer_state.DepthClipEnable));
            state.write_i32(transmute(self.0.rasterizer_state.MultisampleEnable));
            state.write_i32(transmute(self.0.rasterizer_state.AntialiasedLineEnable));
            state.write_u32(self.0.rasterizer_state.ForcedSampleCount);
            state.write_i32(transmute(self.0.rasterizer_state.ConservativeRaster));
        }
        // Depth & stencil
        unsafe {
            state.write_i32(transmute(self.0.depth_stencil_state.DepthEnable));
            state.write_i32(transmute(self.0.depth_stencil_state.DepthWriteMask));
            state.write_i32(transmute(self.0.depth_stencil_state.DepthFunc));
            state.write_i32(transmute(self.0.depth_stencil_state.StencilEnable));
            state.write_u8(self.0.depth_stencil_state.StencilReadMask);
            state.write_u8(self.0.depth_stencil_state.StencilWriteMask);

            state.write_i32(transmute(
                self.0.depth_stencil_state.FrontFace.StencilFailOp,
            ));
            state.write_i32(transmute(
                self.0.depth_stencil_state.FrontFace.StencilDepthFailOp,
            ));
            state.write_i32(transmute(
                self.0.depth_stencil_state.FrontFace.StencilPassOp,
            ));
            state.write_i32(transmute(self.0.depth_stencil_state.FrontFace.StencilFunc));

            state.write_i32(transmute(self.0.depth_stencil_state.BackFace.StencilFailOp));
            state.write_i32(transmute(
                self.0.depth_stencil_state.BackFace.StencilDepthFailOp,
            ));
            state.write_i32(transmute(self.0.depth_stencil_state.BackFace.StencilPassOp));
            state.write_i32(transmute(self.0.depth_stencil_state.BackFace.StencilFunc));
        }

        unsafe {
            state.write_i32(transmute(*self.0.primitive_topology_type));
        }

        for i in 0..self.0.rtv_formats.NumRenderTargets {
            unsafe {
                state.write_u32(transmute(self.0.rtv_formats.RTFormats[i as usize]));
            }
        }

        unsafe {
            state.write_u32(transmute(*self.0.dsv_format));
        }
    }
}

impl From<&GraphicsPipelineStateDesc> for GraphicsPipelineEntry {
    fn from(desc: &GraphicsPipelineStateDesc) -> Self {
        Self(desc.clone())
    }
}

#[derive(Default)]
pub struct PipelineManager {
    graphics_pipelines:
        RwLock<HashMap<GraphicsPipelineEntry, SendableIUnknown<ID3D12PipelineState>>>,
}

impl PipelineManager {
    pub fn get_or_create_graphics_pipeline(
        &self,
        device: &ID3D12Device2,
        desc: &GraphicsPipelineStateDesc,
    ) -> ID3D12PipelineState {
        let graphics_pipelines = self.graphics_pipelines.read();
        let entry = desc.into();
        if let Some(pipeline) = graphics_pipelines.get(&entry) {
            pipeline.deref().clone()
        } else {
            drop(graphics_pipelines);

            let mut graphics_pipelines = self.graphics_pipelines.write();

            let pipeline: ID3D12PipelineState = {
                if desc.vertex_shader.BytecodeLength > 0 {
                    let stream = VertexGraphicsPipelineStateDescStream {
                        root_signature: desc.root_signature.clone(),
                        vertex_shader: desc.vertex_shader.clone(),
                        pixel_shader: desc.pixel_shader.clone(),
                        stream_output: desc.stream_output.clone(),
                        blend_state: desc.blend_state.clone(),
                        sample_mask: desc.sample_mask,
                        rasterizer_state: desc.rasterizer_state,
                        depth_stencil_state: desc.depth_stencil_state.clone(),
                        input_layout: desc.input_layout.clone(),
                        ib_strip_cut_value: desc.ib_strip_cut_value.clone(),
                        primitive_topology_type: desc.primitive_topology_type.clone(),
                        rtv_formats: desc.rtv_formats.clone(),
                        dsv_format: desc.dsv_format.clone(),
                        sample_desc: desc.sample_desc.clone(),
                        node_mask: desc.node_mask,
                        cached_pso: desc.cached_pso.clone(),
                        flags: desc.flags.clone(),
                    };

                    let stream_desc = D3D12_PIPELINE_STATE_STREAM_DESC {
                        pPipelineStateSubobjectStream: &stream as *const _ as *mut _,
                        SizeInBytes: size_of_val(&stream),
                    };

                    unsafe { device.CreatePipelineState(&stream_desc) }.unwrap()
                } else {
                    let stream = MeshGraphicsPipelineStateDescStream {
                        root_signature: desc.root_signature.clone(),
                        mesh_shader: desc.mesh_shader.clone(),
                        pixel_shader: desc.pixel_shader.clone(),
                        stream_output: desc.stream_output.clone(),
                        blend_state: desc.blend_state.clone(),
                        sample_mask: desc.sample_mask,
                        rasterizer_state: desc.rasterizer_state,
                        depth_stencil_state: desc.depth_stencil_state.clone(),
                        input_layout: desc.input_layout.clone(),
                        ib_strip_cut_value: desc.ib_strip_cut_value.clone(),
                        primitive_topology_type: desc.primitive_topology_type.clone(),
                        rtv_formats: desc.rtv_formats.clone(),
                        dsv_format: desc.dsv_format.clone(),
                        sample_desc: desc.sample_desc.clone(),
                        node_mask: desc.node_mask,
                        cached_pso: desc.cached_pso.clone(),
                        flags: desc.flags.clone(),
                        amplification_shader: desc.amplification_shader.clone(),
                    };

                    let stream_desc = D3D12_PIPELINE_STATE_STREAM_DESC {
                        pPipelineStateSubobjectStream: &stream as *const _ as *mut _,
                        SizeInBytes: size_of_val(&stream),
                    };

                    unsafe { device.CreatePipelineState(&stream_desc) }.unwrap()
                }
            };

            graphics_pipelines.insert(entry, pipeline.clone().into());
            pipeline
        }
    }
}
