use cfg_if::cfg_if;
use std::path::Path;

fn main() {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));

    let out_dir = cmake::Config::new(manifest_dir.join("third-party/freetype")).build();
    println!(
        "cargo:rustc-link-search=native={}",
        out_dir.join("lib").display()
    );

    cfg_if! {
        if #[cfg(debug_assertions)] {
            println!("cargo:rustc-link-lib=static=freetyped");
        } else {
            println!("cargo:rustc-link-lib=static=freetype");
        }
    }

    println!("cargo:rerun-if-changed=freetype-bindgen.h");

    let bindings = bindgen::Builder::default()
        .header(manifest_dir.join("freetype-bindgen.h").to_string_lossy())
        .clang_arg(format!(
            "-I{}",
            out_dir.join("include/freetype2").to_string_lossy()
        ))
        .parse_callbacks(Box::new(bindgen::CargoCallbacks))
        .generate()
        .expect("Failed to generate bindings");

    let out_path = manifest_dir.join("src");
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings.");
}
