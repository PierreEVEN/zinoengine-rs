use parking_lot::RwLock;
use std::fmt::{Display, Formatter};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Represents a filesystem, containing multiple mount points
///
/// Mounts points might reference actual directories, pak files, network location, etc
pub struct FileSystem {
    mount_points: RwLock<Vec<Box<dyn MountPoint>>>,
}

#[derive(Debug)]
pub enum FileSystemError {
    NotFound,
    PermissionDenied,
}

impl Display for FileSystemError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Debug)]
pub enum WatchEvent {
    Write(PathBuf),
}

pub trait MountPoint: Send + Sync {
    fn read(&self, path: &Path) -> Result<Box<dyn Read>, FileSystemError>;
    fn iter_dir(&self, path: &Path, f: &dyn Fn(&Path)) -> Result<(), FileSystemError>;
    fn watch(
        &self,
        path: &Path,
        f: &Arc<(dyn Fn(WatchEvent) + Send + Sync + 'static)>,
    ) -> Result<(), FileSystemError>;
}

impl FileSystem {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            mount_points: RwLock::new(vec![]),
        })
    }

    pub fn mount(&self, mount_point: Box<dyn MountPoint>) {
        self.mount_points.write().push(mount_point);
    }

    pub fn read(&self, path: &Path) -> Result<Box<dyn Read>, FileSystemError> {
        let mount_point_guard = self.mount_points.read();
        for mount_point in mount_point_guard.iter() {
            let result = mount_point.read(path);
            match result {
                Ok(file) => return Ok(file),
                Err(error) => match error {
                    FileSystemError::NotFound => continue,
                    _ => return Err(error),
                },
            }
        }

        Err(FileSystemError::NotFound)
    }

    pub fn iter_dir(&self, path: &Path, f: impl Fn(&Path)) -> Result<(), FileSystemError> {
        let mount_point_guard = self.mount_points.read();
        for mount_point in mount_point_guard.iter() {
            let result = mount_point.iter_dir(path, &f);
            match result {
                Ok(_) => return Ok(()),
                Err(error) => match error {
                    FileSystemError::NotFound => continue,
                    _ => return Err(error),
                },
            }
        }

        Err(FileSystemError::NotFound)
    }

    pub fn watch<F>(&self, path: &Path, f: F) -> Result<(), FileSystemError>
    where
        F: Fn(WatchEvent) + Send + Sync + 'static,
    {
        let func: Arc<dyn Fn(WatchEvent) + Send + Sync + 'static> = Arc::new(f);
        let mount_point_guard = self.mount_points.read();
        for mount_point in mount_point_guard.iter() {
            let result = mount_point.watch(path, &func);
            match result {
                Ok(_) => return Ok(()),
                Err(error) => match error {
                    FileSystemError::NotFound => continue,
                    _ => return Err(error),
                },
            }
        }

        Err(FileSystemError::NotFound)
    }
}

pub mod mount_points;
