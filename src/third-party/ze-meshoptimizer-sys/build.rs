use std::path::Path;

fn main() {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));

    let cpp_files = [
        "third-party/meshoptimizer/src/allocator.cpp",
        "third-party/meshoptimizer/src/clusterizer.cpp",
        "third-party/meshoptimizer/src/indexcodec.cpp",
        "third-party/meshoptimizer/src/indexgenerator.cpp",
        "third-party/meshoptimizer/src/overdrawanalyzer.cpp",
        "third-party/meshoptimizer/src/overdrawoptimizer.cpp",
        "third-party/meshoptimizer/src/simplifier.cpp",
        "third-party/meshoptimizer/src/spatialorder.cpp",
        "third-party/meshoptimizer/src/stripifier.cpp",
        "third-party/meshoptimizer/src/vcacheanalyzer.cpp",
        "third-party/meshoptimizer/src/vcacheoptimizer.cpp",
        "third-party/meshoptimizer/src/vertexcodec.cpp",
        "third-party/meshoptimizer/src/vertexfilter.cpp",
        "third-party/meshoptimizer/src/vfetchanalyzer.cpp",
        "third-party/meshoptimizer/src/vfetchoptimizer.cpp",
    ];

    for file in cpp_files.iter() {
        let path = manifest_dir.join(file);
        println!("cargo:rerun-if-changed={}", path.display());
    }
    println!("cargo:rerun-if-changed=third-party/meshoptimizer/src/meshoptimizer.h");

    cc::Build::new()
        .cpp(true)
        .files(cpp_files)
        .compile("meshoptimizer");

    let bindings = bindgen::Builder::default()
        .header(
            manifest_dir
                .join("third-party/meshoptimizer/src/meshoptimizer.h")
                .to_string_lossy(),
        )
        .parse_callbacks(Box::new(bindgen::CargoCallbacks))
        .generate_comments(true)
        .generate()
        .expect("Failed to generate bindings");

    let out_path = manifest_dir.join("src");
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings.");
}
