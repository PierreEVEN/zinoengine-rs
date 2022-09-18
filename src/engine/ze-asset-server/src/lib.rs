use parking_lot::{Mutex, RwLock};
use serde_derive::{Deserialize, Serialize};
use sha2::Digest;
use sha2::Sha256;
use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use std::io::Cursor;
use std::path::Path;
use std::sync::Arc;
use url::Url;
use uuid::Uuid;
use ze_asset_system::importer::BoxedAssetImporter;
use ze_asset_system::{AssetLoadResult, AssetProvider, LoadError, ASSET_METADATA_EXTENSION};
use ze_core::{ze_error, ze_info};
use ze_filesystem::{FileSystem, IterDirFlagBits, IterDirFlags};

#[derive(Debug)]
pub enum Error {
    CannotCreateOrOpenSourceDb,
    CannotCreateOrOpenAssetDb,
    UnknownAsset,
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Serialize, Deserialize)]
struct SourceAssetDbEntry {
    source_hash_sha256: Vec<u8>,
}

/// Asset server, this track source assets and import & store them in a cache
/// Used for editor/dev environment only
///
/// Asset source file information are stored inside the source database (source.db)
pub struct AssetServer {
    filesystem: Arc<FileSystem>,
    importers: RwLock<HashMap<String, Arc<dyn BoxedAssetImporter>>>,
    asset_dirs: Mutex<Vec<Url>>,
    source_db: sled::Db,
    asset_db: sled::Db,
}

impl AssetServer {
    pub fn new(
        filesystem: Arc<FileSystem>,
        asset_dirs: Vec<Url>,
        cache_url: Url,
    ) -> Result<Self, Error> {
        // Create or load our source asset database
        let source_db = {
            let mut url = cache_url.clone();
            url.path_segments_mut().unwrap().push("source-db");

            let path = match filesystem.to_underlying_path(&url) {
                Ok(path) => path,
                Err(_) => return Err(Error::CannotCreateOrOpenSourceDb),
            };

            match sled::open(path) {
                Ok(db) => db,
                Err(_) => return Err(Error::CannotCreateOrOpenSourceDb),
            }
        };

        // Same for the asset database
        let asset_db = {
            let mut url = cache_url;
            url.path_segments_mut().unwrap().push("asset-db");

            let path = match filesystem.to_underlying_path(&url) {
                Ok(path) => path,
                Err(_) => return Err(Error::CannotCreateOrOpenSourceDb),
            };

            match sled::open(path) {
                Ok(db) => db,
                Err(_) => return Err(Error::CannotCreateOrOpenSourceDb),
            }
        };

        let server = Self {
            filesystem,
            importers: Default::default(),
            asset_dirs: Default::default(),
            source_db,
            asset_db,
        };

        server.add_asset_paths(&asset_dirs);
        Ok(server)
    }

    pub fn add_asset_paths(&self, paths: &[Url]) {
        let mut asset_dirs = self.asset_dirs.lock();
        for path in paths {
            asset_dirs.push(path.clone());
        }
    }

    pub fn scan_asset_directories(&self) {
        let asset_dirs = self.asset_dirs.lock();
        for path in asset_dirs.iter() {
            self.filesystem
                .iter_dir(
                    path,
                    IterDirFlags::from_flag(IterDirFlagBits::Recursive),
                    |entry| {
                        self.process_potential_source_asset(&entry.url);
                    },
                )
                .unwrap_or_else(|_| ze_error!("Failed to scan asset directory {}", path));
        }
    }

    pub fn add_importer<T>(&self, extensions: &[&str], importer: T)
    where
        T: BoxedAssetImporter + 'static,
    {
        let mut importers = self.importers.write();
        let importer = Arc::new(importer);

        for extension in extensions {
            importers.insert(extension.to_string(), importer.clone());
        }

        drop(importers);
        self.scan_asset_directories();
    }

    pub fn asset_data(&self, uuid: Uuid) -> Result<(Uuid, Vec<u8>), Error> {
        let data = match self.asset_db.get(uuid) {
            Ok(data) => data.unwrap(),
            Err(_) => return Err(Error::UnknownAsset),
        };

        let type_uuid_bytes = match self.asset_db.get(format!("{}_type_uuid", uuid.as_u128())) {
            Ok(data) => data.unwrap(),
            Err(_) => return Err(Error::UnknownAsset),
        };

        let type_uuid = Uuid::from_slice(&type_uuid_bytes).unwrap();
        Ok((type_uuid, data.to_vec()))
    }

    pub fn is_extension_importable(&self, extension: &str) -> bool {
        self.importers.read().get(extension).is_some()
    }

    fn process_potential_source_asset(&self, url: &Url) {
        let path = Path::new(url.path());
        if let Some(extension) = path.extension() {
            if let Some(importer) = self.importer_for_extension(&extension.to_string_lossy()) {
                let key = url.as_str();

                let current_file_hash = {
                    let mut hasher = Sha256::new();
                    let mut file = self.filesystem.read(url).unwrap();
                    std::io::copy(&mut file, &mut hasher).unwrap();
                    hasher.finalize()
                };

                // Ask the source DB more information about this file to check if we should import
                // the asset
                if let Ok(Some(entry)) = self.source_db.get(key) {
                    let mut entry: SourceAssetDbEntry =
                        bincode::serde::decode_from_slice(&entry, bincode::config::standard())
                            .expect("source database maybe corrupted!")
                            .0;

                    if entry.source_hash_sha256.as_slice() != current_file_hash.as_slice() {
                        entry.source_hash_sha256 = current_file_hash.to_vec();
                        self.import_source_asset(url, entry, importer);
                    }
                } else {
                    let entry = SourceAssetDbEntry {
                        source_hash_sha256: current_file_hash.to_vec(),
                    };
                    self.import_source_asset(url, entry, importer);
                }
            }
        }
    }

    fn import_source_asset(
        &self,
        url: &Url,
        source_db_entry: SourceAssetDbEntry,
        importer: Arc<dyn BoxedAssetImporter>,
    ) {
        ze_info!("Importing {}", url.to_string());

        let metadata_url = {
            let mut url = url.clone();
            let asset_path =
                url.path().to_string().rsplit('.').collect::<Vec<&str>>()[1].to_string();
            let path = format!("{}.{}", asset_path, ASSET_METADATA_EXTENSION);
            url.set_path(&path);
            url
        };

        let mut file = self.filesystem.read(url).unwrap();
        match importer.import(&self.filesystem, url, &mut file, &metadata_url) {
            Ok(assets) => {
                for asset in assets {
                    self.asset_db
                        .insert(asset.uuid(), asset.data().clone())
                        .expect("Failed to store asset to asset database correctly!");

                    self.asset_db
                        .insert(
                            format!("{}_type_uuid", asset.uuid().as_u128()),
                            asset.type_uuid().as_bytes(),
                        )
                        .expect("Failed to store asset to asset database correctly!");
                }

                self.source_db
                    .insert(
                        url.as_str(),
                        bincode::serde::encode_to_vec(source_db_entry, bincode::config::standard())
                            .expect("Cannot encode source asset database correctly!"),
                    )
                    .expect("Failed to insert to source db");
            }
            Err(error) => {
                ze_error!("Failed to import asset {}: {:?}", url, error);
            }
        };
    }

    pub fn asset_type_uuid(&self, uuid: Uuid) -> Option<Uuid> {
        let type_uuid_bytes = match self.asset_db.get(format!("{}_type_uuid", uuid.as_u128())) {
            Ok(data) => data.unwrap(),
            Err(_) => return None,
        };

        Some(Uuid::from_slice(&type_uuid_bytes).unwrap())
    }

    pub fn asset_uuid_from_url(&self, url: &Url) -> Option<Uuid> {
        let metadata_url = {
            let mut url = url.clone();
            let asset_path =
                url.path().to_string().rsplit('.').collect::<Vec<&str>>()[1].to_string();
            let path = format!("{}.{}", asset_path, ASSET_METADATA_EXTENSION);
            url.set_path(&path);
            url
        };

        #[derive(Deserialize)]
        struct Metadata {
            uuid: Uuid,
        }

        if let Ok(file) = self.filesystem.read(&metadata_url) {
            if let Ok(metadata) = serde_yaml::from_reader(file) {
                let metadata: Metadata = metadata;
                return Some(metadata.uuid);
            }
        }

        None
    }

    fn importer_for_extension(&self, extension: &str) -> Option<Arc<dyn BoxedAssetImporter>> {
        let importers = self.importers.read();
        importers.get(extension).cloned()
    }
}

pub struct AssetServerProvider {
    asset_server: Arc<AssetServer>,
}

impl AssetServerProvider {
    pub fn new(asset_server: Arc<AssetServer>) -> Self {
        Self { asset_server }
    }
}

impl AssetProvider for AssetServerProvider {
    fn load(&self, uuid: Uuid, _: &Url) -> Result<AssetLoadResult, LoadError> {
        match self.asset_server.asset_data(uuid) {
            Ok(data) => Ok(AssetLoadResult::Serialized(
                data.0,
                Box::new(Cursor::new(data.1)),
            )),
            Err(_) => Err(LoadError::NotFound),
        }
    }
}
