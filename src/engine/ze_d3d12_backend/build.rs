use std::path::Path;

fn main() {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));

    cc::Build::new()
        .cpp(true)
        .include("third-party\\WinPixEventRuntime\\include\\")
        .file("pix-wrapper.cpp")
        .compile("ze_d3d12_backend_pix_wrapper");

    println!(
        "cargo:rustc-link-search={}\\third-party\\WinPixEventRuntime\\bin\\x64",
        env!("CARGO_MANIFEST_DIR")
    );
    println!("cargo:rustc-link-lib=WinPixEventRuntime");
    println!("cargo:rerun-if-changed=pix-wrapper.hpp");
    println!("cargo:rerun-if-changed=pix-wrapper.cpp");
    println!("cargo:rustc-link-lib=static=ze_d3d12_backend_pix_wrapper");

    let bindings = bindgen::Builder::default()
        .header(manifest_dir.join("pix-wrapper.hpp").to_string_lossy())
        .parse_callbacks(Box::new(bindgen::CargoCallbacks))
        .layout_tests(false)
        .generate()
        .expect("Failed to generate bindings");

    let out_path = manifest_dir.join("src");
    bindings
        .write_to_file(out_path.join("pix.rs"))
        .expect("Couldn't write bindings.");
}
