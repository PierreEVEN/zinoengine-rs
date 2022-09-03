#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(clippy::all)]

use std::ops::Add;

include!("./bindings.rs");

impl ImVec2 {
    pub fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
}

impl Default for ImVec2 {
    fn default() -> Self {
        Self { x: 0.0, y: 0.0 }
    }
}

impl From<f32> for ImVec2 {
    fn from(f: f32) -> Self {
        Self { x: f, y: f }
    }
}

impl Add for ImVec2 {
    type Output = ImVec2;

    fn add(self, rhs: Self) -> Self::Output {
        Self::Output {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
        }
    }
}

impl ImVec4 {
    pub fn new(x: f32, y: f32, z: f32, w: f32) -> Self {
        Self { x, y, z, w }
    }
}

impl From<f32> for ImVec4 {
    fn from(f: f32) -> Self {
        Self {
            x: f,
            y: f,
            z: f,
            w: 1.0,
        }
    }
}
