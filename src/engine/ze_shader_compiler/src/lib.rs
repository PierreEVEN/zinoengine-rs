use ze_gfx::ShaderStageFlagBits;

pub struct ShaderCompilerInput<'a> {
    pub name: &'a str,
    pub stage: ShaderStageFlagBits,
    pub code: &'a [u8],
    pub entry_point: &'a str,
}

pub struct ShaderCompilerOutput {
    pub bytecode: Vec<u8>,
}

pub trait ShaderCompiler: Send + Sync {
    fn compile_shader(
        &self,
        input: ShaderCompilerInput,
    ) -> Result<ShaderCompilerOutput, Vec<String>>;
}

impl ShaderCompilerOutput {
    pub fn new(bytecode: Vec<u8>) -> Self {
        Self { bytecode }
    }
}
