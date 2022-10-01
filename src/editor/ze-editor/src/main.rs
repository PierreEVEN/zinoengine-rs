use crate::editor::EditorApplication;
use ze_core::logger::StdoutSink;
use ze_core::{logger, thread};

#[cfg(target_os = "windows")]
ze_d3d12_backend::ze_d3d12_agility_sdk_statics!();

use mimalloc::MiMalloc;

#[global_allocator]
static GLOBAL_ALLOCATOR: MiMalloc = MiMalloc;

fn main() {
    puffin::set_scopes_on(true);
    thread::set_thread_name(std::thread::current().id(), "Main Thread".to_string());
    logger::register_sink(StdoutSink::new());

    let _server = puffin_http::Server::new("127.0.0.1:8585").unwrap();

    let mut editor = EditorApplication::new();
    editor.run();
}

mod asset_explorer;
mod console;
mod editor;
mod icon_manager;
