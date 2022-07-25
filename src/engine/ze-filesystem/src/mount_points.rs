use crate::{
    make_url_for_zefs, DirEntry, DirEntryType, Error, IterDirFlagBits, IterDirFlags, MountPoint,
    WatchEvent,
};
use notify::{DebouncedEvent, RecommendedWatcher, RecursiveMode, Watcher};
use parking_lot::Mutex;
use std::collections::HashMap;
use std::fs::{read_dir, File};
use std::io::{ErrorKind, Read, Write};
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::mpsc::channel;
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use url::Url;

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
    pub fn new(alias: &str, root: &Path) -> Box<Self> {
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
                                        let path = StdMountPoint::fs_path_to_zefs_path(
                                            &root,
                                            &path.canonicalize().unwrap(),
                                        );
                                        f(WatchEvent::Write(
                                            make_url_for_zefs(&alias, &path).unwrap(),
                                        ));
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

    fn fs_path_to_zefs_path(root: &str, path: &Path) -> String {
        let mut path = path.to_string_lossy().to_string().replace('\\', "/");
        path.replace_range(0..root.len(), "");
        path
    }

    fn to_fs_path(&self, path: &Url) -> PathBuf {
        // Because we use long paths we can't pass to std::fs functions any forward slashes
        // Only for Windows
        #[cfg(windows)]
        let path = path.path().replace('/', "\\");

        let path = format!("{}{}", self.root.to_string_lossy(), path);
        PathBuf::from_str(&path).unwrap()
    }
}

impl MountPoint for StdMountPoint {
    fn read(&self, path: &Url) -> Result<Box<dyn Read>, Error> {
        let file = File::open(self.to_fs_path(path))?;
        Ok(Box::new(file))
    }

    fn write(&self, path: &Url) -> Result<Box<dyn Write>, Error> {
        let file = File::create(self.to_fs_path(path))?;
        Ok(Box::new(file))
    }

    fn iter_dir(
        &self,
        path: &Url,
        flags: IterDirFlags,
        f: &mut dyn FnMut(DirEntry),
    ) -> Result<(), Error> {
        let dir = read_dir(self.to_fs_path(path))?;

        for entry in dir {
            let entry = entry?;

            let path = Self::fs_path_to_zefs_path(
                &self.root.to_string_lossy(),
                &entry.path().clone().canonicalize().unwrap(),
            );
            let path = make_url_for_zefs(self.alias(), &path).unwrap();

            let file_type = entry.file_type().unwrap();
            let entry = DirEntry {
                ty: if file_type.is_dir() {
                    DirEntryType::Directory
                } else {
                    DirEntryType::File
                },
                url: path,
            };

            f(entry.clone());
            if flags.contains(IterDirFlagBits::Recursive) && entry.ty == DirEntryType::Directory {
                self.iter_dir(&entry.url, flags, f)?;
            }
        }

        Ok(())
    }

    fn watch(
        &self,
        path: &Url,
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

    fn to_underlying_path(&self, url: &Url) -> Result<PathBuf, Error> {
        Ok(self.to_fs_path(url))
    }
}
