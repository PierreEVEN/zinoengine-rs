use crate::loader::AssetLoader;
use parking_lot::RwLock;
use std::any::Any;
use std::collections::HashMap;
use std::io::Read;
use std::str::FromStr;
use std::sync::{Arc, Weak};
use url::Url;
use uuid::{Error, Uuid};
use ze_core::downcast_rs::{impl_downcast, DowncastSync};
use ze_core::ze_info;

pub const ASSET_METADATA_EXTENSION: &str = "zeassetmeta";

pub trait Asset: Any + DowncastSync {
    fn uuid(&self) -> Uuid;
}
impl_downcast!(Asset);

pub trait AssetProvider {
    fn load(&self, uuid: Uuid, url: &Url) -> Result<(Uuid, Box<dyn Read>), LoadError>;
}

#[derive(Debug, Eq, PartialEq)]
pub enum LoadError {
    InvalidUrl,
    NotFound,
    NoViableLoader,
    LoaderError(loader::Error),
}

impl From<uuid::Error> for LoadError {
    fn from(_: Error) -> Self {
        Self::InvalidUrl
    }
}

#[derive(Default)]
pub struct AssetManager {
    providers: RwLock<Vec<(Box<dyn AssetProvider>, u32)>>,
    loaders: RwLock<HashMap<Uuid, Arc<dyn AssetLoader>>>,
    asset_cache: RwLock<HashMap<Uuid, Weak<dyn Asset>>>,
}

impl AssetManager {
    pub fn load(&self, url: &Url) -> Result<Arc<dyn Asset>, LoadError> {
        // Try searching the cache first
        let asset_uuid = {
            let asset_cache = self.asset_cache.read();
            let path = url.path().split_at(1).1;
            let uuid = Uuid::from_str(path)?;
            if let Some(asset) = asset_cache.get(&uuid) {
                if let Some(asset) = asset.upgrade() {
                    return Ok(asset);
                }
            }
            uuid
        };

        let providers = self.providers.read();
        for (provider, _) in providers.iter() {
            match provider.load(asset_uuid, url) {
                Ok((asset_type_uuid, mut asset)) => {
                    let loader = match self.get_loader_for_type_uuid(asset_type_uuid) {
                        Some(loader) => loader,
                        None => return Err(LoadError::NoViableLoader),
                    };

                    ze_info!("Loading {}", url);
                    return match loader.load(asset_uuid, &mut asset) {
                        Ok(asset) => Ok(asset),
                        Err(error) => Err(LoadError::LoaderError(error)),
                    };
                }
                Err(error) => {
                    if error != LoadError::NotFound {
                        return Err(error);
                    }
                }
            }
        }

        Err(LoadError::NotFound)
    }

    pub fn add_provider<P: AssetProvider + 'static>(&self, provider: P, priority: u32) {
        let mut providers = self.providers.write();
        providers.push((Box::new(provider), priority));
        providers.sort_by(|a, b| a.1.cmp(&b.1).reverse());
    }

    pub fn add_loader<L: AssetLoader + 'static>(&self, type_uuid: Uuid, loader: L) {
        let mut loaders = self.loaders.write();
        loaders.insert(type_uuid, Arc::new(loader));
    }

    pub fn get_loader_for_type_uuid(&self, uuid: Uuid) -> Option<Arc<dyn AssetLoader>> {
        self.loaders.read().get(&uuid).cloned()
    }
}

pub mod importer;
pub mod loader;
