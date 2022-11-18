use std::path::Path;

fn main() {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));

    println!("cargo:rerun-if-changed=third-party/D3D12MemoryAllocator/src/D3D12MemAlloc.cpp");
    println!("cargo:rerun-if-changed=D3D12MemoryAllocator.cpp");

    let mut build = cc::Build::new();
    build
        .cpp(true)
        .include(manifest_dir.join("third-party/D3D12MemoryAllocator/include/"))
        .file(manifest_dir.join("D3D12MemoryAllocator.cpp"));

    #[cfg(debug_assertions)]
    build.debug(true);
    build.compile("D3D12MemoryAllocator");

    let bindings = bindgen::Builder::default()
        .header(
            manifest_dir
                .join("third-party/D3D12MemoryAllocator/include/D3D12MemAlloc.h")
                .to_string_lossy(),
        )
        .parse_callbacks(Box::new(bindgen::CargoCallbacks))
        .clang_arg("-x")
        .clang_arg("c++")
        .allowlist_type("D3D12MA::Allocator")
        .allowlist_type("D3D12MA::Allocation")
        .allowlist_type("D3D12MA::Pool")
        .allowlist_function("D3D12MA::CreateAllocator")
        .layout_tests(false) // FIXME: Disable layouts test for now because it fails on std::atomic
        .generate_comments(true)
        .generate()
        .expect("Failed to generate bindings");

    let out_path = manifest_dir.join("src");
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings.");
}
