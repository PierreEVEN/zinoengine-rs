use lazy_static::lazy_static;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;
use std::thread::ThreadId;

lazy_static! {
    static ref THREAD_NAME_MAP: RwLock<HashMap<ThreadId, Arc<String>>> =
        RwLock::new(HashMap::new());
}

pub fn set_thread_name(id: ThreadId, name: String) {
    THREAD_NAME_MAP.write().insert(id, Arc::new(name));
}

pub fn thread_name(id: ThreadId) -> Option<Arc<String>> {
    THREAD_NAME_MAP.read().get(&id).cloned()
}
