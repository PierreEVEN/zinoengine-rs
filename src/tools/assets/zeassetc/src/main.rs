use clap::{arg, Command};
use libloading::Library;
use serde::Deserialize;
use std::env;
use std::fs::read_dir;
use std::path::Path;

#[derive(Deserialize)]
struct CompilerInfo {
    name: String,
    version: String,
    inputs: Vec<String>,
    output: String,
}

struct Compiler {
    info: CompilerInfo,
    _lib: Library,
    compile_func: unsafe extern "C" fn() -> bool,
}

impl Compiler {
    fn can_compile(&self, path: &Path) -> bool {
        let extension = path.extension().unwrap();
        for input in &self.info.inputs {
            if input.as_str() == extension.to_string_lossy() {
                return true;
            }
        }

        false
    }
}

fn discover_compilers() -> Vec<Compiler> {
    let mut compilers = vec![];

    let mut exe_dir = env::current_exe().unwrap();
    exe_dir.pop();

    let paths = read_dir(exe_dir).unwrap();
    for path in paths {
        let path = path.unwrap().path();
        if path.extension().is_some() && path.extension().unwrap() == env::consts::DLL_EXTENSION {
            let lib = unsafe { Library::new(path.clone()) };
            match lib {
                Ok(lib) => {
                    if let Ok(get_toml_func) = unsafe {
                        lib.get::<unsafe extern "C" fn() -> *const str>(b"zeassetc_get_toml\0")
                    } {
                        if let Ok(compile_func) = unsafe {
                            lib.get::<unsafe extern "C" fn() -> bool>(b"zeassetc_compile\0")
                        } {
                            let toml = unsafe { get_toml_func() };
                            let compiler_info =
                                toml::from_str::<CompilerInfo>(unsafe { toml.as_ref() }.unwrap())
                                    .unwrap_or_else(|_| {
                                        panic!("dll {} has wrong TOML!", path.display())
                                    });
                            let compile_func = *compile_func;
                            compilers.push(Compiler {
                                info: compiler_info,
                                _lib: lib,
                                compile_func,
                            })
                        }
                    }
                }
                Err(err) => {
                    panic!("Failed to load library {}: {}", path.display(), err);
                }
            }
        }
    }

    compilers
}

fn main() {
    let matches = Command::new("zeassetc")
        .version(env!("CARGO_PKG_VERSION"))
        .about("Compiles a raw asset")
        .subcommand(
            Command::new("list-compilers").about("List all compilers and there input/output files"),
        )
        .subcommand(
            Command::new("compile")
                .about("Compile a asset")
                .arg(arg!(--input <FILE>)),
        )
        .get_matches();

    let compilers = discover_compilers();

    let find_compiler = |path: &Path| -> Option<&Compiler> {
        for compiler in &compilers {
            if compiler.can_compile(path) {
                return Some(compiler);
            }
        }

        None
    };

    if matches.subcommand_matches("list-compilers").is_some() {
        println!("List of available compiler(s):");
        for compiler in &compilers {
            println!(
                "\t- {} v{} ({:?} -> {})",
                compiler.info.name,
                compiler.info.version,
                compiler.info.inputs,
                compiler.info.output
            );
        }
    } else if let Some(matches) = matches.subcommand_matches("compile") {
        if let Some(file) = matches.get_one::<String>("input") {
            let file = Path::new(file);
            if file.exists() && file.is_file() {
                if let Some(compiler) = find_compiler(file) {
                    println!("Compiling {} using {}", file.display(), compiler.info.name);
                    unsafe {
                        (compiler.compile_func)();
                    }
                } else {
                    eprintln!("No compiler found for asset {}", file.display());
                }
            } else {
                eprintln!("{} is not a valid path or file!", file.display());
            }
        }
    }
}
