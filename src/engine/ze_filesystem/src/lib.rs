use parking_lot::RwLock;
use std::fmt::{Display, Formatter};
use std::io::Read;
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
    Write(Url),
}

pub trait MountPoint: Send + Sync {
    fn read(&self, path: &Url) -> Result<Box<dyn Read>, FileSystemError>;
    fn iter_dir(&self, path: &Url, f: &dyn Fn(&Url)) -> Result<(), FileSystemError>;
    fn watch(
        &self,
        path: &Url,
        f: &Arc<(dyn Fn(WatchEvent) + Send + Sync + 'static)>,
    ) -> Result<(), FileSystemError>;
    fn get_alias(&self) -> &str;
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
            alias = mount_point.get_alias()
        );
        self.mount_points.write().push(mount_point);
    }

    pub fn read(&self, path: &Url) -> Result<Box<dyn Read>, FileSystemError> {
        if let Some(index) = self.get_matching_mount_point_for_url(path) {
            let mount_point_guard = self.mount_points.read();
            mount_point_guard[index].read(path)
        } else {
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
    }

    pub fn iter_dir(&self, path: &Url, f: impl Fn(&Url)) -> Result<(), FileSystemError> {
        if let Some(index) = self.get_matching_mount_point_for_url(path) {
            let mount_point_guard = self.mount_points.read();
            mount_point_guard[index].iter_dir(path, &f)
        } else {
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
    }

    pub fn watch<F>(&self, path: &Url, f: F) -> Result<(), FileSystemError>
    where
        F: Fn(WatchEvent) + Send + Sync + 'static,
    {
        let func: Arc<dyn Fn(WatchEvent) + Send + Sync + 'static> = Arc::new(f);

        if let Some(index) = self.get_matching_mount_point_for_url(path) {
            let mount_point_guard = self.mount_points.read();
            mount_point_guard[index].watch(path, &func)
        } else {
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

    fn get_matching_mount_point_for_url(&self, url: &Url) -> Option<usize> {
        if let Some(Host::Domain(domain)) = url.host() {
            let mount_points = self.mount_points.read();
            for (index, mount_point) in mount_points.iter().enumerate() {
                if mount_point.get_alias() == domain {
                    return Some(index);
                }
            }
        }

        None
    }
}

pub mod mount_points;

pub fn make_url_for_zefs(mount_point: &str, path: &str) -> Result<Url, ParseError> {
    let url = format!("vfs://{}{}", mount_point, path);
    Url::from_str(&url)
}
