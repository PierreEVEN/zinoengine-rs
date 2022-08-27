use serde_derive::{Deserialize, Serialize};
use std::sync::Arc;
use ze_asset_system::Asset;
use ze_core::type_uuid::*;
use ze_gfx::backend::ShaderResourceView;
use ze_gfx::{backend, PixelFormat};
use ze_reflection::*;

#[derive(Serialize, Deserialize, TypeUuid, Default, Reflectable)]
#[type_uuid = "55642466-cdee-450a-885b-72c355dd8713"]
pub struct Texture {
    #[serde(skip_serializing, skip_deserializing)]
    uuid: Uuid,

    #[ze_reflect]
    width: u32,

    #[ze_reflect]
    height: u32,

    #[ze_reflect]
    depth: u32,

    #[ze_reflect]
    format: PixelFormat,

    mip_levels: Vec<Vec<u8>>,

    #[serde(skip_serializing, skip_deserializing)]
    texture: Option<Arc<backend::Texture>>,

    #[serde(skip_serializing, skip_deserializing)]
    default_srv: Option<Arc<ShaderResourceView>>,
}

impl Texture {
    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    pub fn depth(&self) -> u32 {
        self.depth
    }

    pub fn format(&self) -> PixelFormat {
        self.format
    }

    pub fn mip_levels(&self) -> &Vec<Vec<u8>> {
        &self.mip_levels
    }

    pub fn texture(&self) -> &Option<Arc<backend::Texture>> {
        &self.texture
    }

    pub fn default_srv(&self) -> &Option<Arc<ShaderResourceView>> {
        &self.default_srv
    }
}

impl Asset for Texture {
    fn uuid(&self) -> Uuid {
        self.uuid
    }
}

pub mod importer;
pub mod loader;
