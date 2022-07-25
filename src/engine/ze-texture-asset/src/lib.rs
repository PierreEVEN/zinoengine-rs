use serde_derive::{Deserialize, Serialize};
use ze_gfx::PixelFormat;

#[derive(Serialize, Deserialize)]
pub struct Texture {
    width: u32,
    height: u32,
    depth: u32,
    format: PixelFormat,
    mip_levels: Vec<Vec<u8>>,
}

pub mod importer;
pub mod loader;
