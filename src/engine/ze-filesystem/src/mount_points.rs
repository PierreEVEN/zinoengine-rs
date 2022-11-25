use crate::path::{Path as ZefsPath, Path};
use crate::DirEntry;
use crate::DirEntryType;
use crate::Error;
use crate::IterDirFlagBits;
use crate::IterDirFlags;
use crate::MountPoint;
use crate::WatchEvent;
use notify::{DebouncedEvent, RecommendedWatcher, RecursiveMode, Watcher};
use parking_lot::Mutex;
use std::collections::HashMap;
use std::fs::{read_dir, File};
use std::io::{ErrorKind, Read, Write};
use std::path::PathBuf;
use std::sync::mpsc::channel;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        match err.kind() {
            ErrorKind::NotFound => Error::NotFound,
            ErrorKind::PermissionDenied => Error::PermissionDenied,
            _ => panic!("Cannot convert {:?} error kind", err.kind()),
        }
    }
}

impl From<notify::Error> for Error {
    fn from(err: notify::Error) -> Self {
        match err {
            notify::Error::PathNotFound => Error::NotFound,
            _ => panic!("Cannot convert {:?} error kind", err),
        }
    }
}

type StdMountPointWatcher = Arc<(dyn Fn(WatchEvent) + Send + Sync + 'static)>;

pub struct StdMountPoint {
    alias: String,
    root: PathBuf,
    watcher: Mutex<RecommendedWatcher>,
    watcher_closure_map: Arc<Mutex<HashMap<PathBuf, StdMountPointWatcher>>>,
}

impl StdMountPoint {
    pub fn new(alias: &str, root: &std::path::Path) -> Box<Self> {
        let (tx, rx) = channel();
        let watcher = Watcher::new(tx, Duration::from_millis(100)).unwrap();
        let watcher_closure_map: Arc<Mutex<HashMap<PathBuf, StdMountPointWatcher>>> =
            Arc::new(Mutex::new(HashMap::new()));
        let root = root.canonicalize().unwrap();

        {
            let watcher_closure_map = watcher_closure_map.clone();
            let alias = alias.to_string();
            let root = root.to_string_lossy().to_string();
            thread::Builder::new()
                .name("IO Watcher Thread (StdMountPoint)".to_string())
                .spawn(move || {
                    ze_core::thread::set_thread_name(
                        thread::current().id(),
                        "IO Watcher Thread".to_string(),
                    );
                    loop {
                        if let Ok(event) = rx.recv() {
                            match event {
                                DebouncedEvent::Write(path) => {
                                    let watcher_closure_map = watcher_closure_map.lock();
                                    if let Some(f) = watcher_closure_map.get(&path) {
                                        let path = Self::fs_path_to_zefs_path(
                                            root.as_ref(),
                                            &path.canonicalize().unwrap(),
                                        );
                                        f(WatchEvent::Write(ZefsPath::from_mount_point_and_path(
                                            &alias, &path,
                                        )));
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                })
                .unwrap();
        }

        Box::new(Self {
            alias: alias.to_string(),
            root,
            watcher: Mutex::new(watcher),
            watcher_closure_map,
        })
    }

    fn fs_path_to_zefs_path(root: &std::path::Path, path: &std::path::Path) -> String {
        let path = path.to_string_lossy();
        let path = &path
            .strip_prefix(std::path::MAIN_SEPARATOR)
            .unwrap_or(&path)[root.as_os_str().len()..];

        let path = path.to_string();

        #[cfg(windows)]
        let path = path.replace('\\', "/");

        path
    }

    fn to_fs_path(&self, path: &Path) -> PathBuf {
        let path = path.path();

        // Because we use long paths we can't pass to std::fs functions any forward slashes
        // Only for Windows
        #[cfg(windows)]
        let path = path.replace('/', "\\");

        let mut path_buf = PathBuf::with_capacity(self.root.as_os_str().len() + path.len());
        path_buf.push(self.root.as_path());
        path_buf.push(path);
        path_buf
    }
}

impl MountPoint for StdMountPoint {
    fn exists(&self, path: &Path) -> bool {
        let path = self.to_fs_path(path);
        path.exists()
    }

    fn read(&self, path: &Path) -> Result<Box<dyn Read>, Error> {
        let file = File::open(self.to_fs_path(path))?;
        Ok(Box::new(file))
    }

    fn write(&self, path: &Path) -> Result<Box<dyn Write>, Error> {
        let file = File::create(self.to_fs_path(path))?;
        Ok(Box::new(file))
    }

    fn iter_dir(
        &self,
        path: &Path,
        flags: IterDirFlags,
        f: &mut dyn FnMut(&DirEntry),
    ) -> Result<(), Error> {
        let dir = read_dir(self.to_fs_path(path))?;

        for entry in dir {
            let entry = entry?;

            let path = Self::fs_path_to_zefs_path(self.root.as_path(), &entry.path());
            let path = Path::from_mount_point_and_path(self.alias(), &path);

            let file_type = entry.file_type().unwrap();
            let entry = DirEntry {
                ty: if file_type.is_dir() {
                    DirEntryType::Directory
                } else {
                    DirEntryType::File
                },
                path,
            };

            f(&entry);
            if flags.contains(IterDirFlagBits::Recursive) && entry.ty == DirEntryType::Directory {
                self.iter_dir(&entry.path, flags, f)?;
            }
        }

        Ok(())
    }

    fn watch(
        &self,
        path: &Path,
        f: &Arc<dyn Fn(WatchEvent) + Send + Sync + 'static>,
    ) -> Result<(), Error> {
        let path = self.to_fs_path(path).canonicalize().unwrap();
        self.watcher
            .lock()
            .watch(path.clone(), RecursiveMode::NonRecursive)?;
        self.watcher_closure_map.lock().insert(path, f.clone());
        Ok(())
    }

    fn alias(&self) -> &str {
        &self.alias
    }

    fn to_underlying_path(&self, path: &Path) -> Result<PathBuf, Error> {
        Ok(self.to_fs_path(path))
    }
}
