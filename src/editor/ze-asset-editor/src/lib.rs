use parking_lot::Mutex;
use std::collections::HashMap;
use ze_core::type_uuid::Uuid;
use ze_imgui::ze_imgui_sys::ImGuiID;

pub trait AssetEditor {
    fn draw(&self, imgui: &mut ze_imgui::Context);
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
        let editors = self.editors.lock();
        for editor in editors.iter() {
            imgui.next_window_dock_id(main_dockspace_id);
            editor.draw(imgui);
        }
    }
}
