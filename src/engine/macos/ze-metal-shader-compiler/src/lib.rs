use std::sync::Arc;
use ze_shader_compiler::ShaderCompiler;

pub struct MetalShaderCompiler {}

impl MetalShaderCompiler {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {})
    }
}

impl ShaderCompiler for MetalShaderCompiler {
    fn compile_shader(
        &self,
        input: ze_shader_compiler::ShaderCompilerInput,
    ) -> Result<ze_shader_compiler::ShaderCompilerOutput, Vec<String>> {
        todo!()
    }
}
