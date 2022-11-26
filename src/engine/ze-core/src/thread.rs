use fnv::FnvHashMap;
use once_cell::sync::Lazy;
use parking_lot::RwLock;
use std::sync::Arc;
use std::thread::ThreadId;

static THREAD_NAME_MAP: Lazy<RwLock<FnvHashMap<ThreadId, Arc<String>>>> =
    Lazy::new(RwLock::default);

pub fn set_thread_name(id: ThreadId, name: String) {
    THREAD_NAME_MAP.write().insert(id, Arc::new(name));
}

pub fn thread_name(id: ThreadId) -> Option<Arc<String>> {
    THREAD_NAME_MAP.read().get(&id).cloned()
}
