use enumflags2::*;
use parking_lot::RwLock;
use std::fmt::{Display, Formatter};
use std::io::{Read, Write};
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::Arc;
use url::{Host, ParseError, Url};
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
    Write(Url),
}

#[derive(PartialEq, Eq, Copy, Clone)]
pub enum DirEntryType {
    File,
    Directory,
}

#[derive(PartialEq, Eq, Clone)]
pub struct DirEntry {
    pub ty: DirEntryType,
    pub url: Url,
}

#[bitflags]
#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Debug)]
#[repr(u32)]
pub enum IterDirFlagBits {
    Recursive = 1 << 0,
}

pub type IterDirFlags = BitFlags<IterDirFlagBits>;

pub trait MountPoint: Send + Sync {
    fn read(&self, path: &Url) -> Result<Box<dyn Read>, Error>;
    fn write(&self, path: &Url) -> Result<Box<dyn Write>, Error>;
    fn iter_dir(
        &self,
        path: &Url,
        flags: IterDirFlags,
        f: &mut dyn FnMut(DirEntry),
    ) -> Result<(), Error>;
    fn watch(
        &self,
        path: &Url,
        f: &Arc<(dyn Fn(WatchEvent) + Send + Sync + 'static)>,
    ) -> Result<(), Error>;
    fn alias(&self) -> &str;
    fn to_underlying_path(&self, url: &Url) -> Result<PathBuf, Error>;
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
            "Mounted \"{alias}\": vfs://{alias}/",
            alias = mount_point.alias()
        );
        self.mount_points.write().push(mount_point);
    }

    pub fn read(&self, path: &Url) -> Result<Box<dyn Read>, Error> {
        if let Some(index) = self.matching_mount_point_for_url(path) {
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

    pub fn write(&self, path: &Url) -> Result<Box<dyn Write>, Error> {
        if let Some(index) = self.matching_mount_point_for_url(path) {
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
        path: &Url,
        flags: IterDirFlags,
        mut f: impl FnMut(DirEntry),
    ) -> Result<(), Error> {
        if let Some(index) = self.matching_mount_point_for_url(path) {
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

    pub fn watch<F>(&self, path: &Url, f: F) -> Result<(), Error>
    where
        F: Fn(WatchEvent) + Send + Sync + 'static,
    {
        let func: Arc<dyn Fn(WatchEvent) + Send + Sync + 'static> = Arc::new(f);

        if let Some(index) = self.matching_mount_point_for_url(path) {
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

    /// Get the underlying path from the URL. The mount point must be indicated in the URL.
    pub fn to_underlying_path(&self, url: &Url) -> Result<PathBuf, Error> {
        if let Some(index) = self.matching_mount_point_for_url(url) {
            let mount_point_guard = self.mount_points.read();
            mount_point_guard[index].to_underlying_path(url)
        } else {
            Err(Error::UnknownMountPoint)
        }
    }

    fn matching_mount_point_for_url(&self, url: &Url) -> Option<usize> {
        if let Some(Host::Domain(domain)) = url.host() {
            let mount_points = self.mount_points.read();
            for (index, mount_point) in mount_points.iter().enumerate() {
                if mount_point.alias() == domain {
                    return Some(index);
                }
            }
        }

        None
    }
}

pub fn make_url_for_zefs(mount_point: &str, path: &str) -> Result<Url, ParseError> {
    let url = format!("vfs://{}{}", mount_point, path);
    Url::from_str(&url)
}

pub mod mount_points;
pub extern crate percent_encoding;
pub extern crate url;
