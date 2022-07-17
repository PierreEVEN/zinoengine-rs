use crate::editor::EditorApplication;
use ze_core::logger::StdoutSink;
use ze_core::{logger, thread};

ze_d3d12_backend::ze_d3d12_agility_sdk_statics!();

fn main() {
    thread::set_thread_name(std::thread::current().id(), "Main Thread".to_string());
    logger::register_sink(Box::new(StdoutSink::default()));

    let mut editor = EditorApplication::new();
    editor.run();
}

mod asset_explorer;
mod editor;
