use crate::{FileSystemError, MountPoint, WatchEvent};
use notify::{DebouncedEvent, RecommendedWatcher, RecursiveMode, Watcher};
use parking_lot::Mutex;
use std::collections::HashMap;
use std::fs::{read_dir, File};
use std::io::{Error, ErrorKind, Read};
use std::path::{Path, PathBuf};
use std::sync::mpsc::channel;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

impl From<Error> for FileSystemError {
    fn from(err: Error) -> Self {
        match err.kind() {
            ErrorKind::NotFound => FileSystemError::NotFound,
            ErrorKind::PermissionDenied => FileSystemError::PermissionDenied,
            _ => panic!("Cannot convert {:?} error kind", err.kind()),
        }
    }
}

impl From<notify::Error> for FileSystemError {
    fn from(err: notify::Error) -> Self {
        match err {
            notify::Error::PathNotFound => FileSystemError::NotFound,
            _ => panic!("Cannot convert {:?} error kind", err),
        }
    }
}

pub struct StdMountPoint {
    root: PathBuf,
    watcher: Mutex<RecommendedWatcher>,
    watcher_closure_map:
        Arc<Mutex<HashMap<PathBuf, Arc<(dyn Fn(WatchEvent) + Send + Sync + 'static)>>>>,
}

impl StdMountPoint {
    pub fn new(root: &Path) -> Box<Self> {
        let (tx, rx) = channel();
        let watcher = Watcher::new(tx, Duration::from_millis(100)).unwrap();
        let watcher_closure_map: Arc<
            Mutex<HashMap<PathBuf, Arc<(dyn Fn(WatchEvent) + Send + Sync + 'static)>>>,
        > = Arc::new(Mutex::new(HashMap::new()));

        {
            let watcher_closure_map = watcher_closure_map.clone();
            thread::Builder::new()
                .name("IO Watcher Thread (StdMountPoint)".to_string())
                .spawn(move || {
                    ze_core::thread::set_thread_name(
                        thread::current().id(),
                        "IO Watcher Thread".to_string(),
                    );
                    loop {
                        match rx.recv() {
                            Ok(event) => match event {
                                DebouncedEvent::Write(path) => {
                                    let watcher_closure_map = watcher_closure_map.lock();
                                    if let Some(f) = watcher_closure_map.get(&path) {
                                        f(WatchEvent::Write(path));
                                    }
                                }
                                _ => {}
                            },
                            Err(_) => {}
                        }
                    }
                })
                .unwrap();
        }

        Box::new(Self {
            root: root.to_path_buf(),
            watcher: Mutex::new(watcher),
            watcher_closure_map,
        })
    }

    fn correct_path(&self, path: &Path) -> PathBuf {
        if path.is_relative() {
            let mut correct_path = self.root.clone();
            correct_path.push(path);
            return correct_path;
        }

        path.to_path_buf()
    }
}

impl MountPoint for StdMountPoint {
    fn read(&self, path: &Path) -> Result<Box<dyn Read>, FileSystemError> {
        let file = File::open(self.correct_path(path))?;
        Ok(Box::new(file))
    }

    fn iter_dir(&self, path: &Path, f: &dyn Fn(&Path)) -> Result<(), FileSystemError> {
        let dir = read_dir(self.correct_path(path))?;

        for entry in dir {
            let entry = entry?;
            if entry.path().is_dir() {
                self.iter_dir(entry.path().as_path(), f)?;
            } else {
                f(entry.path().as_path());
            }
        }

        Ok(())
    }

    fn watch(
        &self,
        path: &Path,
        f: &Arc<dyn Fn(WatchEvent) + Send + Sync + 'static>,
    ) -> Result<(), FileSystemError> {
        let path = self.correct_path(path).canonicalize().unwrap();
        self.watcher
            .lock()
            .watch(path.clone(), RecursiveMode::NonRecursive)?;
        self.watcher_closure_map.lock().insert(path, f.clone());
        Ok(())
    }
}
