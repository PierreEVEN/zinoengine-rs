pub(crate) struct D3D12ShaderModule {
    pub bytecode: Vec<u8>,
}

impl D3D12ShaderModule {
    pub fn new(bytecode: Vec<u8>) -> Self {
        Self { bytecode }
    }
}
