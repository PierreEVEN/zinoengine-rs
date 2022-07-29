use crate::editor::EditorApplication;
use ze_core::logger::StdoutSink;
use ze_core::{logger, thread};

ze_d3d12_backend::ze_d3d12_agility_sdk_statics!();

fn main() {
    thread::set_thread_name(std::thread::current().id(), "Main Thread".to_string());
    logger::register_sink(StdoutSink::new());

    let mut editor = EditorApplication::new();
    editor.run();
}

mod asset_explorer;
mod console;
mod editor;
mod icon_manager;
