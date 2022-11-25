use parking_lot::{Mutex, RwLock};
use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::ffi::OsStr;
use std::hash::Hasher;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use ze_core::signals::SyncSignal;
use ze_core::sparse_vec::SparseVec;
use ze_core::{ze_error, ze_info};
use ze_filesystem::path::Path;
use ze_filesystem::{FileSystem, IterDirFlagBits, IterDirFlags, WatchEvent};
use ze_gfx::backend::{Device, PipelineShaderStage, ShaderModule};
use ze_gfx::ShaderStageFlagBits;
use ze_jobsystem::JobSystem;
use ze_shader_compiler::{ShaderCompiler, ShaderCompilerInput};

enum ShaderStageSourceData {
    _Bytecode(Vec<u8>),
    Hlsl(String),
}

pub struct ShaderStage {
    stage: ShaderStageFlagBits,
    source_data: ShaderStageSourceData,
}

impl ShaderStage {
    fn new(stage: ShaderStageFlagBits, source_data: ShaderStageSourceData) -> Self {
        Self { stage, source_data }
    }
}

pub struct ShaderPass {
    name: String,
    stages: Vec<ShaderStage>,
}

impl ShaderPass {
    fn new(name: String, stages: Vec<ShaderStage>) -> Self {
        Self { name, stages }
    }
}

#[derive(Eq, PartialEq, Debug)]
enum ShaderType {
    Zeshader,
    _ZeshaderBin,
}

pub struct Shader {
    ty: ShaderType,
    name: String,
    passes: Vec<ShaderPass>,
}

impl Shader {
    fn new(ty: ShaderType, name: String, passes: Vec<ShaderPass>) -> Self {
        Self { ty, name, passes }
    }

    fn pass_index(&self, name: &str) -> Option<usize> {
        for (idx, pass) in self.passes.iter().enumerate() {
            if pass.name == name {
                return Some(idx);
            }
        }

        None
    }
}

/// Container of all shaders pipeline stages of a shader pass/permutation
#[derive(Default)]
pub struct ShaderModules {
    stages: Vec<(ShaderStageFlagBits, ShaderModule)>,
}

impl ShaderModules {
    pub fn pipeline_stages(&self) -> Vec<PipelineShaderStage> {
        let mut stages = Vec::with_capacity(self.stages.len());
        for stage in &self.stages {
            stages.push(PipelineShaderStage {
                stage: stage.0,
                module: &stage.1,
            })
        }
        stages
    }
}

/// Simple cache storing the shader modules in a Arc
#[derive(Default)]
struct ShaderModulesCache {
    shaders: RwLock<HashMap<u64, Arc<ShaderModules>>>,
}

impl ShaderModulesCache {
    fn get(&self, id: u64) -> Option<Arc<ShaderModules>> {
        let shaders = self.shaders.read();
        shaders.get(&id).cloned()
    }
}

pub struct CompilingShader {
    name: String,
    bytecodes: Mutex<Vec<(ShaderStageFlagBits, Vec<u8>)>>,
    processed_stages: AtomicUsize,
    stage_count: usize,
    pub on_compiled: SyncSignal<()>,
}

impl CompilingShader {
    fn new(name: String, stage_count: usize) -> Self {
        Self {
            name,
            bytecodes: Default::default(),
            processed_stages: Default::default(),
            stage_count,
            on_compiled: Default::default(),
        }
    }
}

struct CompilationManager {
    jobsystem: Arc<JobSystem>,
    shader_compiler: Arc<dyn ShaderCompiler>,
    shaders: Arc<Mutex<HashMap<u64, Arc<CompilingShader>>>>,
}

impl CompilationManager {
    fn new(jobsystem: Arc<JobSystem>, shader_compiler: Arc<dyn ShaderCompiler>) -> Self {
        Self {
            jobsystem,
            shader_compiler,
            shaders: Default::default(),
        }
    }

    fn is_compiling(&self, id: u64) -> Option<Arc<CompilingShader>> {
        let shaders = self.shaders.lock();
        shaders.get(&id).cloned()
    }

    fn compile_permutation(
        &self,
        key: u64,
        name: &str,
        pass: &ShaderPass,
        callback: impl FnMut(Arc<CompilingShader>) + Clone + Send + Sync + 'static,
    ) -> Arc<CompilingShader> {
        let mut shaders = self.shaders.lock();
        let shader = Arc::new(CompilingShader::new(name.to_string(), pass.stages.len()));
        shaders.insert(key, shader.clone());

        for stage in &pass.stages {
            if let ShaderStageSourceData::Hlsl(code) = &stage.source_data {
                struct CompilationData {
                    shader: Arc<CompilingShader>,
                    code: String,
                    shader_compiler: Arc<dyn ShaderCompiler>,
                    stage_type: ShaderStageFlagBits,
                    callback: Box<dyn FnMut(Arc<CompilingShader>) + Send + Sync + 'static>,
                }

                let shader = shader.clone();
                let code = code.clone();

                let mut compilation_data = Box::new(CompilationData {
                    shader: shader.clone(),
                    code: code.clone(),
                    shader_compiler: self.shader_compiler.clone(),
                    stage_type: stage.stage,
                    callback: Box::new(callback.clone()),
                });

                let shaders = self.shaders.clone();
                self.jobsystem
                    .spawn(move |_, _| {
                        let output =
                            compilation_data
                                .shader_compiler
                                .compile_shader(ShaderCompilerInput {
                                    name: &compilation_data.shader.name,
                                    stage: compilation_data.stage_type,
                                    code: compilation_data.code.as_bytes(),
                                    entry_point: "main",
                                });

                        match output {
                            Ok(output) => {
                                let mut bytecodes = shader.bytecodes.lock();
                                bytecodes.push((compilation_data.stage_type, output.bytecode));
                            }
                            Err(errors) => {
                                let mut error_message = String::new();
                                for error in errors {
                                    error_message.push_str(&error);
                                }

                                ze_error!(
                                    "Failed to compile shader {} stage {:?}: {}",
                                    shader.name,
                                    compilation_data.stage_type,
                                    error_message
                                );
                            }
                        }

                        shader.processed_stages.fetch_add(1, Ordering::SeqCst);
                        if shader.processed_stages.load(Ordering::SeqCst) == shader.stage_count {
                            (*compilation_data.callback)(shader.clone());
                            shaders.lock().remove(&key);
                        }
                    })
                    .schedule();
            } else {
                panic!("Non-HLSL stage in a zeshader file!");
            }
        }

        shader
    }
}

pub enum GetModulesError {
    Compiling(Arc<CompilingShader>),
    Unknown,
}

pub struct ShaderManager {
    device: Arc<dyn Device>,
    shaders: RwLock<SparseVec<Shader>>,
    shader_name_to_index_map: RwLock<HashMap<String, usize>>,
    module_cache: Arc<ShaderModulesCache>,
    compilation_manager: CompilationManager,
}

impl ShaderManager {
    pub fn new(
        device: Arc<dyn Device>,
        jobsystem: Arc<JobSystem>,
        shader_compiler: Arc<dyn ShaderCompiler>,
    ) -> Arc<Self> {
        Arc::new(Self {
            device,
            shaders: RwLock::new(SparseVec::default()),
            shader_name_to_index_map: Default::default(),
            module_cache: Arc::new(ShaderModulesCache::default()),
            compilation_manager: CompilationManager::new(jobsystem, shader_compiler),
        })
    }

    pub fn search_shaders(self: &Arc<ShaderManager>, filesystem: &Arc<FileSystem>, path: &Path) {
        filesystem
            .iter_dir(
                path,
                IterDirFlags::from_flag(IterDirFlagBits::Recursive),
                |entry| {
                    let path = std::path::Path::new(entry.path.path());
                    let extension = path.extension().unwrap_or_else(|| OsStr::new(""));
                    if extension == "zeshader" {
                        if let Ok(()) = self.load_zeshader_file(filesystem, &entry.path) {
                            // Setup a watch for hot-reloading
                            let filesystem_closure = filesystem.clone();
                            let shader_manager = Arc::downgrade(self);
                            filesystem
                                .watch(&entry.path, move |event| {
                                    if let WatchEvent::Write(path) = event {
                                        if let Some(shader_manager) = shader_manager.upgrade() {
                                            shader_manager
                                                .load_zeshader_file(&filesystem_closure, &path)
                                                .unwrap();
                                        }
                                    }
                                })
                                .unwrap();
                        }
                    }
                },
            )
            .unwrap();
    }

    /// Get the modules of the specified shader
    /// If not available yet (compiling) it will returns a signal to know when the shader is ready
    pub fn shader_modules(
        self: &Arc<ShaderManager>,
        name: &String,
        pass: Option<String>,
    ) -> Result<Arc<ShaderModules>, GetModulesError> {
        let shader_name_to_index_map = self.shader_name_to_index_map.read();
        if let Some(shader_index) = shader_name_to_index_map.get(name) {
            let shader_index = *shader_index;
            drop(shader_name_to_index_map); // Drop now so we don't deadlock the IO Watcher Thread and us
            let shader = &self.shaders.read()[shader_index];
            let pass = match &pass {
                None => "",
                Some(name) => name,
            };

            if let Some(pass_idx) = shader.pass_index(pass) {
                let pass = &shader.passes[pass_idx];
                // First search on the cache
                let mut id = DefaultHasher::new();
                id.write_usize(shader_index);
                id.write_usize(pass_idx);
                let id = id.finish();
                if let Some(modules) = self.module_cache.get(id) {
                    Ok(modules)
                } else {
                    assert_eq!(shader.ty, ShaderType::Zeshader);
                    // Find if we are compiling this shader
                    if let Some(shader) = self.compilation_manager.is_compiling(id) {
                        Err(GetModulesError::Compiling(shader))
                    } else {
                        let module_cache = self.module_cache.clone();
                        let name = name.clone();
                        let device = self.device.clone();
                        let shader = self.compilation_manager.compile_permutation(
                            id,
                            &name.clone(),
                            pass,
                            move |shader| {
                                let bytecodes = shader.bytecodes.lock();
                                if bytecodes.len() == shader.stage_count {
                                    ze_info!("Compiled shader {}", name);
                                    let mut shaders = module_cache.shaders.write();
                                    let mut modules = Vec::with_capacity(bytecodes.len());
                                    for (stage, bytecode) in bytecodes.iter() {
                                        let module = device.create_shader_module(bytecode).unwrap();
                                        modules.push((*stage, module));
                                    }
                                    shaders.insert(id, Arc::new(ShaderModules { stages: modules }));
                                }
                            },
                        );
                        Err(GetModulesError::Compiling(shader))
                    }
                }
            } else {
                Err(GetModulesError::Unknown)
            }
        } else {
            Err(GetModulesError::Unknown)
        }
    }

    /// Load a .zeshader shader file into a `Shader`
    fn load_zeshader_file(&self, filesystem: &Arc<FileSystem>, path: &Path) -> Result<(), ()> {
        match self.parse_zeshader_file(filesystem, path) {
            Ok(declaration) => {
                let mut shaders = self.shaders.write();
                for (index, shader) in shaders.iter().enumerate() {
                    if shader.name == declaration.name {
                        let mut cache = self.module_cache.shaders.write();
                        // Remove from cache the shader modules
                        for (pass_idx, _) in shader.passes.iter().enumerate() {
                            let mut id = DefaultHasher::new();
                            id.write_usize(index);
                            id.write_usize(pass_idx);
                            let id = id.finish();
                            cache.remove(&id);
                        }

                        shaders.remove(index);
                        break;
                    }
                }

                // Translate the declaration into a concrete shader
                let mut passes = vec![];
                for pass in declaration.passes {
                    let mut stages = vec![];
                    for stage in pass.stages {
                        let hlsl =
                            declaration.common_hlsl.clone() + &pass.common_hlsl + &stage.hlsl;
                        stages.push(ShaderStage::new(
                            stage.stage,
                            ShaderStageSourceData::Hlsl(hlsl),
                        ));
                    }
                    passes.push(ShaderPass::new(pass.name, stages));
                }

                let shader = Shader::new(ShaderType::Zeshader, declaration.name.clone(), passes);
                ze_info!(
                    "Loaded shader \"{}\" ({} passes/zeshader)",
                    shader.name,
                    shader.passes.len()
                );
                let index = shaders.push(shader);
                let mut shader_name_to_index_map = self.shader_name_to_index_map.write();
                shader_name_to_index_map.insert(declaration.name, index);

                // TODO: Insert into big hashmap

                Ok(())
            }
            Err(err) => {
                ze_error!("Failed to load shader \"{}\": {}", path.as_str(), err);
                Err(())
            }
        }
    }

    fn parse_zeshader_file(
        &self,
        filesystem: &Arc<FileSystem>,
        path: &Path,
    ) -> Result<zeshader::Declaration, String> {
        match filesystem.read(path) {
            Ok(file) => {
                return match zeshader::Declaration::from_read(file) {
                    Ok(decl) => Ok(decl),
                    Err(msg) => Err(format!("Failed to parse zeshader: {}", msg)),
                }
            }
            Err(error) => Err(format!("Failed to read shader ({})", error)),
        }
    }
}

mod zeshader;
