use hassle_rs::{Dxc, DxcIncludeHandler};
use std::io::Read;
use std::sync::Arc;
use ze_filesystem::path::Path;
use ze_filesystem::FileSystem;
use ze_gfx::ShaderStageFlagBits;
use ze_shader_compiler::{ShaderCompiler, ShaderCompilerInput, ShaderCompilerOutput};

struct IncludeHandler<'a> {
    filesystem: &'a Arc<FileSystem>,
}

impl<'a> IncludeHandler<'a> {
    fn new(filesystem: &'a Arc<FileSystem>) -> Self {
        Self { filesystem }
    }
}

impl<'a> DxcIncludeHandler for IncludeHandler<'a> {
    fn load_source(&mut self, filename: String) -> Option<String> {
        let path = "//assets/shaders/".to_string() + &filename;
        if let Ok(mut file) = self.filesystem.read(&Path::parse(&path).unwrap()) {
            let mut content = String::new();
            file.read_to_string(&mut content).unwrap();
            return Some(content);
        }

        None
    }
}

pub struct D3D12ShaderCompiler {
    dxc: Dxc,
    filesystem: Arc<FileSystem>,
}

impl D3D12ShaderCompiler {
    pub fn new(filesystem: Arc<FileSystem>) -> Arc<Self> {
        let dxc = Dxc::new(None).expect("DXC instance cannot be created");
        Arc::new(Self { dxc, filesystem })
    }
}

impl ShaderCompiler for D3D12ShaderCompiler {
    fn compile_shader(
        &self,
        input: ShaderCompilerInput,
    ) -> Result<ShaderCompilerOutput, Vec<String>> {
        let profile = match input.stage {
            ShaderStageFlagBits::Vertex => "vs_6_6",
            ShaderStageFlagBits::Fragment => "ps_6_6",
            ShaderStageFlagBits::Compute => "cs_6_6",
            ShaderStageFlagBits::Mesh => "ms_6_6",
        };

        let compiler = self.dxc.create_compiler().unwrap();
        let library = self.dxc.create_library().unwrap();

        let blob = library.create_blob_with_encoding(input.code).unwrap();

        #[cfg(debug_assertions)]
        let args = ["-Qstrip_reflect", "-WX", "-HV 2021", "-Zi"];

        #[cfg(not(debug_assertions))]
        let args = ["-Qstrip_reflect", "-Qstrip_debug", "-WX", "-HV 2021", "-Zi"];

        let mut include_handler = IncludeHandler::new(&self.filesystem);
        let result = compiler.compile(
            &blob,
            input.name,
            input.entry_point,
            profile,
            &args,
            Some(&mut include_handler),
            &[("ZE_BACKEND_D3D12", Some("1"))],
        );

        match result {
            Ok(result) => {
                let result_blob = result.get_result().unwrap();
                Ok(ShaderCompilerOutput::new(result_blob.to_vec()))
            }
            Err(result) => {
                let error_blob = result.0.get_error_buffer().unwrap();
                Err(vec![library
                    .get_blob_as_string(&error_blob.into())
                    .unwrap()])
            }
        }
    }
}
