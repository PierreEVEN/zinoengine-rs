use cmake::Config;
use std::io;
use std::path::Path;

fn main() -> io::Result<()> {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));

    let cmake = Config::new(manifest_dir.join("third-party/cimgui"))
        .define("CMAKE_BUILD_TYPE", "Release")
        .define("IMGUI_STATIC", "ON")
        .build();

    println!("cargo:rustc-link-search=native={}", cmake.display());
    println!("cargo:rustc-link-lib=static=cimgui");

    let bindings = bindgen::Builder::default()
        .header(manifest_dir.join("cimgui-bindgen.h").to_string_lossy())
        .parse_callbacks(Box::new(bindgen::CargoCallbacks))
        .generate()
        .expect("Failed to generate bindings");

    let out_path = manifest_dir.join("src");
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings.");

    Ok(())
}
