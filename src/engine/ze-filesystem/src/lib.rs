use crate::path::Path;
use enumflags2::*;
use parking_lot::RwLock;
use std::fmt::{Display, Formatter};
use std::io::{Read, Write};
use std::path::PathBuf;
use std::sync::Arc;
use ze_core::ze_info;

/// Represents a filesystem, containing multiple mount points
///
/// Mounts points might reference actual directories, pak files, network location, etc
pub struct FileSystem {
    mount_points: RwLock<Vec<Box<dyn MountPoint>>>,
}

#[derive(Debug)]
pub enum Error {
    NotFound,
    PermissionDenied,
    UnknownMountPoint,
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Debug)]
#[non_exhaustive]
pub enum WatchEvent {
    Write(Path),
}

#[derive(PartialEq, Eq, Copy, Clone)]
pub enum DirEntryType {
    File,
    Directory,
}

#[derive(PartialEq, Eq, Clone)]
pub struct DirEntry {
    pub ty: DirEntryType,
    pub path: Path,
}

#[bitflags]
#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Debug)]
#[repr(u32)]
pub enum IterDirFlagBits {
    Recursive = 1 << 0,
}

pub type IterDirFlags = BitFlags<IterDirFlagBits>;

pub trait MountPoint: Send + Sync {
    fn exists(&self, path: &Path) -> bool;
    fn read(&self, path: &Path) -> Result<Box<dyn Read>, Error>;
    fn write(&self, path: &Path) -> Result<Box<dyn Write>, Error>;
    fn iter_dir(
        &self,
        path: &Path,
        flags: IterDirFlags,
        f: &mut dyn FnMut(&DirEntry),
    ) -> Result<(), Error>;
    fn watch(
        &self,
        path: &Path,
        f: &Arc<(dyn Fn(WatchEvent) + Send + Sync + 'static)>,
    ) -> Result<(), Error>;
    fn alias(&self) -> &str;
    fn to_underlying_path(&self, path: &Path) -> Result<PathBuf, Error>;
}

impl FileSystem {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            mount_points: RwLock::new(vec![]),
        })
    }

    pub fn mount(&self, mount_point: Box<dyn MountPoint>) {
        // TODO: Ensure no mount points shares theirs aliases
        ze_info!(
            "Mounted \"{alias}\": /{alias}/",
            alias = mount_point.alias()
        );
        self.mount_points.write().push(mount_point);
    }

    pub fn exists(&self, path: &Path) -> bool {
        if let Some(index) = self.matching_mount_point_for_path(path) {
            let mount_point_guard = self.mount_points.read();
            mount_point_guard[index].exists(path)
        } else {
            false
        }
    }

    pub fn read(&self, path: &Path) -> Result<Box<dyn Read>, Error> {
        if let Some(index) = self.matching_mount_point_for_path(path) {
            let mount_point_guard = self.mount_points.read();
            mount_point_guard[index].read(path)
        } else {
            let mount_point_guard = self.mount_points.read();
            for mount_point in mount_point_guard.iter() {
                let result = mount_point.read(path);
                match result {
                    Ok(file) => return Ok(file),
                    Err(error) => match error {
                        Error::NotFound => continue,
                        _ => return Err(error),
                    },
                }
            }

            Err(Error::NotFound)
        }
    }

    pub fn write(&self, path: &Path) -> Result<Box<dyn Write>, Error> {
        if let Some(index) = self.matching_mount_point_for_path(path) {
            let mount_point_guard = self.mount_points.read();
            mount_point_guard[index].write(path)
        } else {
            let mount_point_guard = self.mount_points.read();
            for mount_point in mount_point_guard.iter() {
                let result = mount_point.write(path);
                match result {
                    Ok(file) => return Ok(file),
                    Err(error) => match error {
                        Error::NotFound => continue,
                        _ => return Err(error),
                    },
                }
            }

            Err(Error::NotFound)
        }
    }

    pub fn iter_dir(
        &self,
        path: &Path,
        flags: IterDirFlags,
        mut f: impl FnMut(&DirEntry),
    ) -> Result<(), Error> {
        if let Some(index) = self.matching_mount_point_for_path(path) {
            let mount_point_guard = self.mount_points.read();
            mount_point_guard[index].iter_dir(path, flags, &mut f)
        } else {
            let mount_point_guard = self.mount_points.read();
            for mount_point in mount_point_guard.iter() {
                let result = mount_point.iter_dir(path, flags, &mut f);
                match result {
                    Ok(_) => return Ok(()),
                    Err(error) => match error {
                        Error::NotFound => continue,
                        _ => return Err(error),
                    },
                }
            }

            Err(Error::NotFound)
        }
    }

    pub fn watch<F>(&self, path: &Path, f: F) -> Result<(), Error>
    where
        F: Fn(WatchEvent) + Send + Sync + 'static,
    {
        let func: Arc<dyn Fn(WatchEvent) + Send + Sync + 'static> = Arc::new(f);

        if let Some(index) = self.matching_mount_point_for_path(path) {
            let mount_point_guard = self.mount_points.read();
            mount_point_guard[index].watch(path, &func)
        } else {
            let mount_point_guard = self.mount_points.read();
            for mount_point in mount_point_guard.iter() {
                let result = mount_point.watch(path, &func);
                match result {
                    Ok(_) => return Ok(()),
                    Err(error) => match error {
                        Error::NotFound => continue,
                        _ => return Err(error),
                    },
                }
            }

            Err(Error::NotFound)
        }
    }

    pub fn to_underlying_path(&self, path: &Path) -> Result<PathBuf, Error> {
        if let Some(index) = self.matching_mount_point_for_path(path) {
            let mount_point_guard = self.mount_points.read();
            mount_point_guard[index].to_underlying_path(path)
        } else {
            Err(Error::UnknownMountPoint)
        }
    }

    fn matching_mount_point_for_path(&self, path: &Path) -> Option<usize> {
        if let Some(path_mount_point) = path.mount_point() {
            let mount_points = self.mount_points.read();
            for (index, mount_point) in mount_points.iter().enumerate() {
                if mount_point.alias() == path_mount_point {
                    return Some(index);
                }
            }
        }

        None
    }
}

pub mod mount_points;
pub mod path;
