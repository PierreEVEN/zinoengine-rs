use crate::loader::AssetLoader;
use parking_lot::RwLock;
use std::any::Any;
use std::collections::HashMap;
use std::io::Read;
use std::str::FromStr;
use std::sync::{Arc, Weak};
use uuid::{Error, Uuid};
use ze_core::downcast_rs::{impl_downcast, DowncastSync};
use ze_core::type_uuid::DynTypeUuid;
use ze_core::{ze_error, ze_info};
use ze_filesystem::path::Path;

pub const ASSET_METADATA_EXTENSION: &str = "zeassetmeta";

pub trait Asset: Any + DowncastSync + DynTypeUuid {
    fn uuid(&self) -> Uuid;
}
impl_downcast!(Asset);

pub enum AssetLoadResult {
    Loaded(Arc<dyn Asset>),
    Serialized(Uuid, Box<dyn Read>),
}

pub trait AssetProvider {
    fn load(&self, uuid: Uuid, path: &Path) -> Result<AssetLoadResult, LoadError>;
}

#[derive(Debug, Eq, PartialEq)]
pub enum LoadError {
    InvalidPath,
    NotFound,
    NoViableLoader,
    LoaderError(loader::Error),
}

impl From<Error> for LoadError {
    fn from(_: Error) -> Self {
        Self::InvalidPath
    }
}

pub struct AssetManager {
    providers: RwLock<Vec<(Arc<dyn AssetProvider>, u32)>>,
    loaders: RwLock<HashMap<Uuid, Arc<dyn AssetLoader>>>,
    asset_cache: RwLock<HashMap<Uuid, Weak<dyn Asset>>>,
    default_temporary_provider: Arc<TemporaryAssetProvider>,
}

impl Default for AssetManager {
    fn default() -> Self {
        let asset_manager = Self {
            providers: Default::default(),
            loaders: Default::default(),
            asset_cache: Default::default(),
            default_temporary_provider: Arc::new(Default::default()),
        };
        asset_manager.add_provider_arc(asset_manager.default_temporary_provider.clone(), 0);
        asset_manager
    }
}

impl AssetManager {
    pub fn load_sync(&self, path: &Path) -> Result<Arc<dyn Asset>, LoadError> {
        // Try searching the cache first
        let asset_uuid = {
            let asset_cache = self.asset_cache.read();
            let uuid = Uuid::from_str(path.path())?;
            if let Some(asset) = asset_cache.get(&uuid) {
                if let Some(asset) = asset.upgrade() {
                    return Ok(asset);
                }
            }
            uuid
        };

        ze_info!("Loading \"{}\"", path);

        if asset_uuid.is_nil() {
            ze_error!("Invalid asset UUID for \"{}\"", path);
            return Err(LoadError::InvalidPath);
        }

        let providers = self.providers.read();
        for (provider, _) in providers.iter() {
            match provider.load(asset_uuid, path) {
                Ok(result) => {
                    return match result {
                        AssetLoadResult::Loaded(asset) => Ok(asset),
                        AssetLoadResult::Serialized(asset_type_uuid, mut read) => {
                            let loader = match self.loader_for_type_uuid(asset_type_uuid) {
                                Some(loader) => loader,
                                None => return Err(LoadError::NoViableLoader),
                            };

                            return match loader.load(asset_uuid, &mut read) {
                                Ok(asset) => Ok(asset),
                                Err(error) => Err(LoadError::LoaderError(error)),
                            };
                        }
                    }
                }
                Err(error) => {
                    if error != LoadError::NotFound {
                        ze_error!("No provider for \"{}\"", path);
                        return Err(error);
                    }
                }
            }
        }

        Err(LoadError::NotFound)
    }

    pub fn add_provider<P: AssetProvider + 'static>(&self, provider: P, priority: u32) -> Arc<P> {
        let provider = Arc::new(provider);
        self.add_provider_arc(provider.clone(), priority);
        provider
    }

    pub fn add_provider_arc<P: AssetProvider + 'static>(&self, provider: Arc<P>, priority: u32) {
        let mut providers = self.providers.write();
        providers.push((provider, priority));
        providers.sort_by(|a, b| a.1.cmp(&b.1).reverse());
    }

    pub fn add_loader<L: AssetLoader + 'static>(&self, type_uuid: Uuid, loader: L) {
        let mut loaders = self.loaders.write();
        loaders.insert(type_uuid, Arc::new(loader));
    }

    pub fn loader_for_type_uuid(&self, uuid: Uuid) -> Option<Arc<dyn AssetLoader>> {
        self.loaders.read().get(&uuid).cloned()
    }

    /// Returns the default temporary asset provider
    /// Allows to add temporary assets (created by code) to the asset manager
    pub fn temporary_asset_provider(&self) -> &Arc<TemporaryAssetProvider> {
        &self.default_temporary_provider
    }
}

/// Asset provider that provides temporary assets, stored inside a big hashmap
#[derive(Default)]
pub struct TemporaryAssetProvider {
    assets: RwLock<HashMap<Uuid, Arc<dyn Asset>>>,
}

impl TemporaryAssetProvider {
    pub fn add_asset(&self, asset: Arc<dyn Asset>) {
        let mut assets = self.assets.write();
        assets.insert(asset.uuid(), asset);
    }

    pub fn remove_asset(&self, uuid: Uuid) {
        let mut assets = self.assets.write();
        assets.remove(&uuid);
    }
}

impl AssetProvider for TemporaryAssetProvider {
    fn load(&self, uuid: Uuid, _: &Path) -> Result<AssetLoadResult, LoadError> {
        if let Some(asset) = self.assets.read().get(&uuid) {
            Ok(AssetLoadResult::Loaded(asset.clone()))
        } else {
            Err(LoadError::NotFound)
        }
    }
}

pub mod importer;
pub mod loader;
