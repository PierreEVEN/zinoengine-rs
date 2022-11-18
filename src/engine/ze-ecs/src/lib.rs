extern crate core;

mod access;
pub mod archetype;
pub mod component;
pub mod entity;
mod erased_vec;
mod sparse_set;
pub mod system;
pub mod world;

pub use once_cell::sync::Lazy;
pub use ze_ecs_macros::*;

extern crate ze_ecs_macros;
