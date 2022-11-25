use parking_lot::RwLock;
use std::collections::HashMap;
use std::io::Read;
use std::sync::Arc;
use ze_filesystem::path::Path;
use ze_filesystem::FileSystem;
use ze_gfx::backend::*;
use ze_gfx::utils::copy_data_to_texture;
use ze_gfx::PixelFormat;

pub struct Icon {
    pub texture: Arc<Texture>,
    pub srv: Arc<ShaderResourceView>,
}

pub struct IconManager {
    device: Arc<dyn Device>,
    filesystem: Arc<FileSystem>,
    icons: RwLock<HashMap<String, Arc<Icon>>>,
    icon_root_dir: Path,
}

impl IconManager {
    pub fn new(device: Arc<dyn Device>, filesystem: Arc<FileSystem>, icon_root_dir: Path) -> Self {
        Self {
            device,
            filesystem,
            icons: Default::default(),
            icon_root_dir,
        }
    }

    pub fn icon(&self, name: &str) -> Option<Arc<Icon>> {
        let icons = self.icons.read();
        if let Some(icon) = icons.get(name) {
            Some(icon.clone())
        } else {
            drop(icons);
            let path = self.icon_root_dir.clone();
            let path = path.join(&format!("{}{}", name, ".png"));
            if let Ok(mut file) = self.filesystem.read(&path) {
                let mut data = vec![];
                file.read_to_end(&mut data).unwrap();
                let image = image::load_from_memory(&data).unwrap();

                let texture = Arc::new(
                    self.device
                        .create_texture(
                            &TextureDesc {
                                width: image.width(),
                                height: image.height(),
                                depth: 1,
                                mip_levels: 1,
                                format: PixelFormat::R8G8B8A8Unorm,
                                sample_desc: Default::default(),
                                usage_flags: TextureUsageFlags::default(),
                                memory_desc: MemoryDesc {
                                    memory_location: MemoryLocation::GpuOnly,
                                    memory_flags: Default::default(),
                                },
                            },
                            None,
                            "",
                        )
                        .unwrap(),
                );

                let texture_srv = Arc::new(
                    self.device
                        .create_shader_resource_view(&ShaderResourceViewDesc::Texture2D(
                            Texture2DSRV {
                                texture: texture.clone(),
                                format: PixelFormat::R8G8B8A8Unorm,
                                min_mip_level: 0,
                                mip_levels: 1,
                            },
                        ))
                        .unwrap(),
                );

                copy_data_to_texture(
                    &self.device,
                    image.as_bytes(),
                    texture.desc.width,
                    texture.desc.height,
                    4,
                    &texture,
                    ResourceState::Common,
                )
                .unwrap();

                let mut icons = self.icons.write();

                let icon = Arc::new(Icon {
                    texture,
                    srv: texture_srv,
                });
                icons.insert(name.to_string(), icon.clone());
                Some(icon)
            } else {
                None
            }
        }
    }
}
