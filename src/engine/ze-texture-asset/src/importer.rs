﻿use crate::Texture;
use image::EncodableLayout;
use image::{ColorType, ImageFormat};
use num_derive::FromPrimitive;
use num_traits::FromPrimitive;
use serde_derive::{Deserialize, Serialize};
use std::io::Read;
use uuid::Uuid;
use ze_asset_system::importer::{
    AssetImporter, AssetImporterResult, Error, ImportedAsset, SourceAssetMetadata,
};
use ze_core::type_uuid::*;
use ze_filesystem::path::Path;
use ze_gfx::PixelFormat;
use ze_reflection::*;

#[derive(Copy, Clone, Serialize, Deserialize, FromPrimitive, Reflectable)]
pub enum TextureCompressionMode {
    None,

    /// BC1/BC3 or BC6 (HDR) on PC
    NormalQuality,

    /// BC7 or BC6 (HDR) on PC
    HighQuality,

    /// BC3 on PC
    TangentSpaceNormalMap,
}

#[derive(Serialize, Deserialize, Reflectable)]
pub struct Parameters {
    #[ze_reflect(display_name = "Compression Mode")]
    compression_mode: TextureCompressionMode,

    #[ze_reflect(display_name = "sRGB")]
    s_rgb: bool,

    #[ze_reflect(display_name = "Generate Mipmaps")]
    generate_mipmaps: bool,
}

impl Default for Parameters {
    fn default() -> Self {
        Self {
            compression_mode: TextureCompressionMode::HighQuality,
            s_rgb: true,
            generate_mipmaps: false,
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
        src_path: &Path,
        src: &mut dyn Read,
        metadata: Option<SourceAssetMetadata<Self::State, Self::Parameters>>,
    ) -> Result<AssetImporterResult<Self::State, Self::Parameters>, Error> {
        let metadata = match metadata {
            None => SourceAssetMetadata::new(Uuid::new_v4(), (), Parameters::default()),
            Some(metadata) => metadata,
        };

        let format = {
            let extension = src_path
                .path()
                .to_string()
                .rsplit('.')
                .collect::<Vec<&str>>()[0]
                .to_string();

            ImageFormat::from_extension(extension).expect("Unknown source texture format")
        };

        let image = {
            let mut source_data = vec![];
            src.read_to_end(&mut source_data)
                .expect("Failed to read source texture");
            match image::load_from_memory_with_format(&source_data, format) {
                Ok(image) => image,
                Err(_) => return Err(Error::InvalidSourceAsset),
            }
        };

        let format = match image.color() {
            ColorType::Rgb8 | ColorType::Rgba8 => PixelFormat::R8G8B8A8Unorm,
            _ => unimplemented!(),
        };

        let texture = Texture {
            uuid: Default::default(),
            width: image.width(),
            height: image.height(),
            depth: 1,
            format,
            mip_levels: vec![image.to_rgba8().as_bytes().to_vec()],
            texture: None,
            default_srv: None,
        };

        let data = match bincode::serde::encode_to_vec(texture, bincode::config::standard()) {
            Ok(data) => data,
            Err(_) => return Err(Error::FailedToSerialize),
        };

        Ok((
            vec![ImportedAsset::new(
                *metadata.uuid(),
                Texture::type_uuid(),
                data,
            )],
            metadata,
        ))
    }
}
