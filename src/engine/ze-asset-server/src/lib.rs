use parking_lot::{Mutex, RwLock};
use serde_derive::{Deserialize, Serialize};
use sha2::Digest;
use sha2::Sha256;
use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use std::io::Cursor;
use std::sync::Arc;
use uuid::Uuid;
use ze_asset_system::importer::BoxedAssetImporter;
use ze_asset_system::{AssetLoadResult, AssetProvider, LoadError, ASSET_METADATA_EXTENSION};
use ze_core::{ze_error, ze_info};
use ze_filesystem::path::Path;
use ze_filesystem::{DirEntryType, FileSystem, IterDirFlagBits, IterDirFlags};

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
    asset_dirs: Mutex<Vec<Path>>,
    source_db: sled::Db,
    asset_db: sled::Db,
}

impl AssetServer {
    pub fn new(
        filesystem: Arc<FileSystem>,
        asset_dirs: Vec<Path>,
        cache_path: Path,
    ) -> Result<Self, Error> {
        // Create or load our source asset database
        let source_db = {
            let mut path = cache_path.clone();
            path.push("source-db");

            let path = match filesystem.to_underlying_path(&path) {
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
            let mut path = cache_path;
            path.push("asset-db");

            let path = match filesystem.to_underlying_path(&path) {
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

    pub fn add_asset_paths(&self, paths: &[Path]) {
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
                        if entry.ty == DirEntryType::File {
                            self.process_potential_source_asset(&entry.path);
                        }
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

    fn process_potential_source_asset(&self, path: &Path) {
        let key = path.as_str();

        let current_file_hash = {
            let mut hasher = Sha256::new();
            let mut file = self
                .filesystem
                .read(path)
                .unwrap_or_else(|_| panic!("Failed to open file {}", path));
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

            let metadata_path = {
                let mut path = path.clone();
                let asset_path =
                    path.path().to_string().rsplit('.').collect::<Vec<&str>>()[1].to_string();
                let path_str = format!("{}.{}", asset_path, ASSET_METADATA_EXTENSION);
                path.set_path(&path_str);
                path
            };

            if entry.source_hash_sha256.as_slice() != current_file_hash.as_slice()
                || !self.filesystem.exists(&metadata_path)
            {
                entry.source_hash_sha256 = current_file_hash.to_vec();
                if self.import_source_asset(path) {
                    self.source_db
                        .insert(
                            path.as_str(),
                            bincode::serde::encode_to_vec(entry, bincode::config::standard())
                                .expect("Cannot encode source asset database correctly!"),
                        )
                        .expect("Failed to insert to source db");
                }
            }
        } else {
            let entry = SourceAssetDbEntry {
                source_hash_sha256: current_file_hash.to_vec(),
            };

            if self.import_source_asset(path) {
                self.source_db
                    .insert(
                        path.as_str(),
                        bincode::serde::encode_to_vec(entry, bincode::config::standard())
                            .expect("Cannot encode source asset database correctly!"),
                    )
                    .expect("Failed to insert to source db");
            }
        }
    }

    pub fn import_source_asset(&self, path: &Path) -> bool {
        let fs_path = std::path::Path::new(path.path());
        let extension = fs_path.extension().unwrap().to_string_lossy();
        if extension == ASSET_METADATA_EXTENSION {
            return false;
        }

        if let Some(importer) = self.importer_for_extension(&extension) {
            ze_info!("Importing {}", path.to_string());

            let metadata_path = {
                let mut path = path.clone();
                let asset_path =
                    path.path().to_string().rsplit('.').collect::<Vec<&str>>()[1].to_string();
                let path_str = format!("{}.{}", asset_path, ASSET_METADATA_EXTENSION);
                path.set_path(&path_str);
                path
            };

            let mut file = self.filesystem.read(path).unwrap();
            match importer.import(&self.filesystem, path, &mut file, &metadata_path) {
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
                    true
                }
                Err(error) => {
                    ze_error!("Failed to import asset {}: {:?}", path, error);
                    false
                }
            }
        } else {
            ze_error!("No importer for {}", path.to_string());
            false
        }
    }

    pub fn asset_type_uuid(&self, uuid: Uuid) -> Option<Uuid> {
        let type_uuid_bytes = match self.asset_db.get(format!("{}_type_uuid", uuid.as_u128())) {
            Ok(data) => data.unwrap(),
            Err(_) => return None,
        };

        Some(Uuid::from_slice(&type_uuid_bytes).unwrap())
    }

    pub fn asset_uuid_from_path(&self, path: &Path) -> Option<Uuid> {
        let metadata_path = {
            let mut path = path.clone();
            let asset_path =
                path.path().to_string().rsplit('.').collect::<Vec<&str>>()[1].to_string();
            let path_str = format!("{}.{}", asset_path, ASSET_METADATA_EXTENSION);
            path.set_path(&path_str);
            path
        };

        #[derive(Deserialize)]
        struct Metadata {
            uuid: Uuid,
        }

        if let Ok(file) = self.filesystem.read(&metadata_path) {
            if let Ok(metadata) = serde_yaml::from_reader(file) {
                let metadata: Metadata = metadata;
                return Some(metadata.uuid);
            }
        }

        None
    }

    pub fn importer_for_extension(&self, extension: &str) -> Option<Arc<dyn BoxedAssetImporter>> {
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
    fn load(&self, uuid: Uuid, _: &Path) -> Result<AssetLoadResult, LoadError> {
        match self.asset_server.asset_data(uuid) {
            Ok(data) => Ok(AssetLoadResult::Serialized(
                data.0,
                Box::new(Cursor::new(data.1)),
            )),
            Err(_) => Err(LoadError::NotFound),
        }
    }
}
