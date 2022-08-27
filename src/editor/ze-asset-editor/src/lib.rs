use parking_lot::Mutex;
use std::collections::HashMap;
use ze_core::type_uuid::Uuid;
use ze_imgui::ze_imgui_sys::ImGuiID;
use ze_imgui::{WindowFlagBits, WindowFlags};

pub trait AssetEditor {
    fn draw(&mut self, imgui: &mut ze_imgui::Context);
    fn asset_uuid(&self) -> Uuid;
}

pub trait AssetEditorFactory {
    fn open(&self, asset: Uuid) -> Box<dyn AssetEditor>;
}

#[derive(Default)]
pub struct AssetEditorManager {
    editor_factories: Mutex<HashMap<Uuid, Box<dyn AssetEditorFactory>>>,
    editors: Mutex<Vec<Box<dyn AssetEditor>>>,
}

impl AssetEditorManager {
    pub fn add_editor_factory<F: AssetEditorFactory + 'static>(&self, type_uuid: Uuid, editor: F) {
        let mut editors = self.editor_factories.lock();
        editors.insert(type_uuid, Box::new(editor));
    }

    pub fn open_asset(&self, type_uuid: Uuid, uuid: Uuid) {
        let mut editors = self.editors.lock();
        for editor in editors.iter() {
            if editor.asset_uuid() == uuid {
                return;
            }
        }

        let factories = self.editor_factories.lock();
        if let Some(editor) = factories.get(&type_uuid) {
            editors.push(editor.open(uuid));
        }
    }

    pub fn draw_editors(&self, imgui: &mut ze_imgui::Context, main_dockspace_id: ImGuiID) {
        let mut editors = self.editors.lock();
        let mut editors_to_remove = vec![];
        for (i, editor) in editors.iter_mut().enumerate() {
            imgui.next_window_dock_id(main_dockspace_id);
            let mut is_open = true;
            if imgui.begin_window_closable(
                &editor.asset_uuid().to_string(),
                &mut is_open,
                WindowFlags::from_flag(WindowFlagBits::NoSavedSettings),
            ) && is_open
            {
                editor.draw(imgui);
            }

            if !is_open {
                editors_to_remove.push(i);
            }

            imgui.end_window();
        }

        for i in editors_to_remove {
            editors.remove(i);
        }
    }
}
