#![feature(alloc_layout_extra)]
#![feature(sync_unsafe_cell)]
#![feature(ptr_internals)]
#![feature(test)]

extern crate core;
extern crate test;

mod access;
pub mod archetype;
pub mod component;
pub mod entity;
mod erased_vec;
mod sparse_set;
pub mod system;
pub mod world;

extern crate ze_ecs_macros;
