use serde_derive::{Deserialize, Serialize};
use std::io;
use std::io::{Read, Write};
use std::sync::Arc;
use url::Url;
use uuid::Uuid;
use ze_filesystem::FileSystem;

pub struct ImportedAsset {
    uuid: Uuid,
    data: Vec<u8>,
}

impl ImportedAsset {
    pub fn new(uuid: Uuid, data: Vec<u8>) -> Self {
        Self { uuid, data }
    }

    pub fn uuid(&self) -> &Uuid {
        &self.uuid
    }

    pub fn data(&self) -> &Vec<u8> {
        &self.data
    }
}

pub type AssetImporterResult<S, P> = (Vec<ImportedAsset>, SourceAssetMetadata<S, P>);

/// Object capable of importing source assets
pub trait AssetImporter: Send + 'static {
    /// Type storing asset state, serialized into .zeassetmeta file
    type State: serde::Serialize + serde::de::DeserializeOwned + Send + Sync + 'static;

    /// Type storing import parameters, serialized into .zeassetmeta file
    type Parameters: serde::Serialize + serde::de::DeserializeOwned + Send + Sync + 'static;

    fn import(
        &self,
        src_url: &Url,
        src: &mut dyn Read,
        metadata: Option<SourceAssetMetadata<Self::State, Self::Parameters>>,
    ) -> Result<AssetImporterResult<Self::State, Self::Parameters>, Error>;
}

#[derive(Debug)]
pub enum Error {
    CannotWriteMetadata(ze_filesystem::Error),
    IoError(io::Error),
    InvalidYaml(serde_yaml::Error),
    FailedToSerialize,
}

impl From<io::Error> for Error {
    fn from(error: io::Error) -> Self {
        Self::IoError(error)
    }
}

impl From<ze_filesystem::Error> for Error {
    fn from(error: ze_filesystem::Error) -> Self {
        Self::CannotWriteMetadata(error)
    }
}

impl From<serde_yaml::Error> for Error {
    fn from(error: serde_yaml::Error) -> Self {
        Self::InvalidYaml(error)
    }
}

pub trait BoxedAssetImporter: Send + Sync + 'static {
    fn import(
        &self,
        filesystem: &Arc<FileSystem>,
        src_url: &Url,
        src: &mut dyn Read,
        metadata_url: &Url,
    ) -> Result<Vec<ImportedAsset>, Error>;
}

impl<T> BoxedAssetImporter for T
where
    T: AssetImporter + Sync,
{
    fn import(
        &self,
        filesystem: &Arc<FileSystem>,
        src_url: &Url,
        src: &mut dyn Read,
        metadata_url: &Url,
    ) -> Result<Vec<ImportedAsset>, Error> {
        // If we don't have any metadata, we rely on the importer to provide one
        let metadata: Option<SourceAssetMetadata<T::State, T::Parameters>> =
            match filesystem.read(metadata_url) {
                Ok(file) => match serde_yaml::from_reader(file) {
                    Ok(metadata) => metadata,
                    Err(error) => return Err(Error::InvalidYaml(error)),
                },
                Err(_) => None,
            };

        let (assets, metadata) = self.import(src_url, src, metadata)?;

        // Write metadata to the .zeassetmeta file
        let yaml = serde_yaml::to_string(&metadata)?;
        let mut metadata_file = filesystem.write(metadata_url)?;
        metadata_file.write_all(yaml.as_bytes())?;

        Ok(assets)
    }
}

/// Store metadata about a source asset
/// Will typically store the main asset UUID and may also store state (e.g other UUIDs) and import parameters
#[derive(Serialize, Deserialize)]
pub struct SourceAssetMetadata<S, P> {
    uuid: Uuid,
    importer_state: S,
    importer_parameters: P,
}

impl<S, P> SourceAssetMetadata<S, P> {
    pub fn new(uuid: Uuid, importer_state: S, importer_parameters: P) -> Self {
        Self {
            uuid,
            importer_state,
            importer_parameters,
        }
    }

    pub fn uuid(&self) -> &Uuid {
        &self.uuid
    }
}
