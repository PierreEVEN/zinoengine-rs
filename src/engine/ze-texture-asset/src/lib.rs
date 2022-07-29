use serde_derive::{Deserialize, Serialize};
use std::sync::Arc;
use ze_asset_system::Asset;
use ze_core::type_uuid::*;
use ze_gfx::backend::ShaderResourceView;
use ze_gfx::{backend, PixelFormat};

#[derive(Serialize, Deserialize, TypeUuid, Default)]
#[type_uuid = "55642466-cdee-450a-885b-72c355dd8713"]
pub struct Texture {
    #[serde(skip_serializing, skip_deserializing)]
    uuid: Uuid,

    width: u32,
    height: u32,
    depth: u32,
    format: PixelFormat,
    mip_levels: Vec<Vec<u8>>,

    #[serde(skip_serializing, skip_deserializing)]
    texture: Option<Arc<backend::Texture>>,

    #[serde(skip_serializing, skip_deserializing)]
    default_srv: Option<Arc<ShaderResourceView>>,
}

impl Texture {
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
