#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

include!("./bindings.rs");

impl ImVec2 {
    pub fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
}

impl ImVec4 {
    pub fn new(x: f32, y: f32, z: f32, w: f32) -> Self {
        Self { x, y, z, w }
    }
}
