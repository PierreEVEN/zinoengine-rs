use std::path::Path;

fn main() {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));

    println!(
        "cargo:rustc-link-arg=/DEF:{}\\agility.def",
        manifest_dir.to_str().unwrap()
    );
}
