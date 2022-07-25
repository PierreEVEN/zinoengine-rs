use crate::Texture;
use serde_derive::{Deserialize, Serialize};
use std::io::Read;
use url::Url;
use uuid::Uuid;
use ze_asset_system::importer::{
    AssetImporter, AssetImporterResult, Error, ImportedAsset, SourceAssetMetadata,
};
use ze_gfx::PixelFormat;

#[derive(Serialize, Deserialize)]
pub enum TextureCompressionMode {
    None,

    /// BC1/BC3 or BC6 (HDR) on PC
    NormalQuality,

    /// BC7 or BC6 (HDR) on PC
    HighQuality,

    /// BC3 on PC
    TangentSpaceNormalMap,
}

#[derive(Serialize, Deserialize)]
pub struct Parameters {
    compression_mode: TextureCompressionMode,
}

impl Default for Parameters {
    fn default() -> Self {
        Self {
            compression_mode: TextureCompressionMode::HighQuality,
        }
    }
}

#[derive(Default)]
pub struct TextureImporter {}

impl AssetImporter for TextureImporter {
    type State = ();
    type Parameters = Parameters;

    fn import(
        &self,
        src_url: &Url,
        src: &mut dyn Read,
        metadata: Option<SourceAssetMetadata<Self::State, Self::Parameters>>,
    ) -> Result<AssetImporterResult<Self::State, Self::Parameters>, Error> {
        let metadata = match metadata {
            None => SourceAssetMetadata::new(Uuid::new_v4(), (), Parameters::default()),
            Some(metadata) => metadata,
        };

        let texture = Texture {
            width: 256,
            height: 256,
            depth: 1,
            format: PixelFormat::Unknown,
            mip_levels: vec![],
        };

        let data = match bincode::serde::encode_to_vec(texture, bincode::config::standard()) {
            Ok(data) => data,
            Err(_) => return Err(Error::FailedToSerialize),
        };

        Ok((vec![ImportedAsset::new(*metadata.uuid(), data)], metadata))
    }
}
