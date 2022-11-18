use crate::Texture;
use std::io::Read;
use std::sync::Arc;
use uuid::Uuid;
use ze_asset_system::loader::{AssetLoader, Error};
use ze_asset_system::Asset;
use ze_gfx::backend::*;
use ze_gfx::{utils, PixelFormat};

pub struct TextureLoader {
    device: Arc<dyn Device>,
}

impl TextureLoader {
    pub fn new(device: Arc<dyn Device>) -> Self {
        Self { device }
    }
}

impl AssetLoader for TextureLoader {
    fn load(&self, uuid: Uuid, asset: &mut dyn Read) -> Result<Arc<dyn Asset>, Error> {
        let mut data = vec![];
        asset.read_to_end(&mut data).unwrap();

        let mut texture: Texture =
            match bincode::serde::decode_from_slice(&data, bincode::config::standard()) {
                Ok((texture, _)) => texture,
                Err(_) => return Err(Error::CannotDeserialize),
            };

        texture.uuid = uuid;

        texture.texture = match self.device.create_texture(
            &TextureDesc {
                width: texture.width,
                height: texture.height,
                depth: texture.depth,
                mip_levels: texture.mip_levels.len() as u32,
                format: texture.format,
                sample_desc: Default::default(),
                usage_flags: TextureUsageFlags::empty(),
                memory_desc: MemoryDesc {
                    memory_location: MemoryLocation::GpuOnly,
                    memory_flags: Default::default(),
                },
            },
            None,
            &uuid.to_string(),
        ) {
            Ok(texture) => Some(Arc::new(texture)),
            Err(_) => return Err(Error::CannotDeserialize),
        };

        let texture_handle = texture.texture.clone();
        utils::copy_data_to_texture(
            &self.device,
            &texture.mip_levels[0],
            texture.width,
            texture.height,
            texture.format.bytes_size(),
            &texture_handle.unwrap(),
            ResourceState::Common,
        )
        .expect("Cannot copy texture data to GPU");
        let texture_handle = texture.texture.clone();

        texture.default_srv =
            match self
                .device
                .create_shader_resource_view(&ShaderResourceViewDesc::Texture2D(Texture2DSRV {
                    texture: texture_handle.unwrap(),
                    format: PixelFormat::R8G8B8A8Unorm,
                    min_mip_level: 0,
                    mip_levels: texture.mip_levels.len() as u32,
                })) {
                Ok(srv) => Some(Arc::new(srv)),
                Err(_) => return Err(Error::CannotDeserialize),
            };

        Ok(Arc::new(texture))
    }
}
