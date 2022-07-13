extern crate core;

const TOML: &str = include_str!("../zetex_compiler.toml");

#[no_mangle]
pub unsafe extern "C" fn zeassetc_get_toml() -> *const str {
    TOML
}

#[no_mangle]
pub unsafe extern "C" fn zeassetc_compile() -> bool {
    panic!("never again..");
    false
}
