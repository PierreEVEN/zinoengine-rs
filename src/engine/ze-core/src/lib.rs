pub mod color;
pub mod logger;
pub mod maths;
pub mod pool;
pub mod signals;
pub mod sparse_vec;
pub mod sync;
pub mod thread;
pub mod type_uuid;

use mimalloc::MiMalloc;

#[global_allocator]
static GLOBAL_ALLOCATOR: MiMalloc = MiMalloc;

pub extern crate downcast_rs;
