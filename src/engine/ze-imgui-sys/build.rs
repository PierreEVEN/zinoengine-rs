#[cfg(not(target_os = "windows"))]
use cc::Build;
use cmake::Config;
use std::io;
use std::path::Path;

fn main() -> io::Result<()> {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));

    cfg_if::cfg_if! {
        if #[cfg(target_os = "windows")] {
            let cmake = Config::new(manifest_dir.join("third-party/cimgui"))
                .define("CMAKE_BUILD_TYPE", "Release")
                .define("IMGUI_STATIC", "ON")
                .build();

            println!("cargo:rustc-link-search=native={}", cmake.display());
        } else {
            let cimgui_dir = manifest_dir.join("third-party/cimgui");
            let imgui_dir = cimgui_dir.join("imgui");
            let mut build = Build::new();
            build
                .cpp(true)
                .files([
                    cimgui_dir.join("cimgui.cpp"),
                    imgui_dir.join("imgui.cpp"),
                    imgui_dir.join("imgui_demo.cpp"),
                    imgui_dir.join("imgui_draw.cpp"),
                    imgui_dir.join("imgui_tables.cpp"),
                    imgui_dir.join("imgui_widgets.cpp"),
                ])
                .flag("-fno-exceptions")
                .flag("-std=c++17")
                .cpp_set_stdlib("c++")
                .cpp_link_stdlib("c++")
                .compile("cimgui");
        }
    }

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
