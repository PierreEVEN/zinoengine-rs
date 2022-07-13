﻿use crate::{make_url_for_zefs, FileSystemError, MountPoint, WatchEvent};
use notify::{DebouncedEvent, RecommendedWatcher, RecursiveMode, Watcher};
use parking_lot::Mutex;
use std::collections::HashMap;
use std::fs::{read_dir, File};
use std::io::{Error, ErrorKind, Read};
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::mpsc::channel;
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use url::Url;

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
    alias: String,
    root: PathBuf,
    watcher: Mutex<RecommendedWatcher>,
    watcher_closure_map:
        Arc<Mutex<HashMap<PathBuf, Arc<(dyn Fn(WatchEvent) + Send + Sync + 'static)>>>>,
}

impl StdMountPoint {
    pub fn new(alias: &str, root: &Path) -> Box<Self> {
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
                                        f(WatchEvent::Write(Url::from_file_path(path).unwrap()));
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
            alias: alias.to_string(),
            root: root.to_path_buf(),
            watcher: Mutex::new(watcher),
            watcher_closure_map,
        })
    }

    fn get_path(&self, path: &Url) -> PathBuf {
        let path = format!("{}{}", self.root.to_string_lossy(), path.path());
        PathBuf::from_str(&path).unwrap()
    }
}

impl MountPoint for StdMountPoint {
    fn read(&self, path: &Url) -> Result<Box<dyn Read>, FileSystemError> {
        let file = File::open(self.get_path(path))?;
        Ok(Box::new(file))
    }

    fn iter_dir(&self, path: &Url, f: &dyn Fn(&Url)) -> Result<(), FileSystemError> {
        let dir = read_dir(self.get_path(path))?;

        for entry in dir {
            let entry = entry?;
            let mut path = entry.path().clone().to_string_lossy().to_string();
            let root = self.root.to_string_lossy();

            // Filter out root
            path.replace_range(0..root.len(), "");

            let path = make_url_for_zefs(self.get_alias(), &path).unwrap();
            if entry.path().is_dir() {
                self.iter_dir(&path, f)?;
            } else {
                f(&path);
            }
        }

        Ok(())
    }

    fn watch(
        &self,
        path: &Url,
        f: &Arc<dyn Fn(WatchEvent) + Send + Sync + 'static>,
    ) -> Result<(), FileSystemError> {
        let path = self.get_path(path).canonicalize().unwrap();
        self.watcher
            .lock()
            .watch(path.clone(), RecursiveMode::NonRecursive)?;
        self.watcher_closure_map.lock().insert(path, f.clone());
        Ok(())
    }

    fn get_alias(&self) -> &str {
        &self.alias
    }
}