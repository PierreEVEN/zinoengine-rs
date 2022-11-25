use enumflags2::*;
use parking_lot::Mutex;
use std::collections::HashMap;
use std::sync::Arc;
use ze_asset_server::AssetServer;
use ze_asset_system::ASSET_METADATA_EXTENSION;
use ze_core::type_uuid::Uuid;
use ze_core::ze_info;
use ze_filesystem::path::Path;
use ze_filesystem::FileSystem;
use ze_imgui::ze_imgui_sys::{ImGuiID, ImVec2};
use ze_imgui::{Cond, Context, Key, WindowFlagBits, WindowFlags};

pub trait AssetEditor {
    fn draw(&mut self, imgui: &mut Context, context: &mut AssetEditorDrawContext);
    fn save(&self, filesystem: &FileSystem) -> bool;
    fn asset_uuid(&self) -> Uuid;
}

pub trait AssetEditorFactory {
    fn open(
        &self,
        filesystem: &FileSystem,
        asset: Uuid,
        source_path: &Path,
        metadata_path: &Path,
    ) -> Option<Box<dyn AssetEditor>>;
}

struct AssetEditorEntry {
    editor: Box<dyn AssetEditor>,
    source_path: Path,
    unsaved: bool,
    closing: bool,
    closed: bool,
}

impl AssetEditorEntry {
    pub fn new(editor: Box<dyn AssetEditor>, source_path: Path) -> Self {
        Self {
            editor,
            source_path,
            unsaved: false,
            closing: false,
            closed: false,
        }
    }
}

pub struct AssetEditorManager {
    filesystem: Arc<FileSystem>,
    asset_server: Arc<AssetServer>,
    editor_factories: Mutex<HashMap<Uuid, Box<dyn AssetEditorFactory>>>,
    editors: Mutex<Vec<AssetEditorEntry>>,
}

#[derive(Default)]
pub struct AssetEditorDrawContext {
    need_save: bool,
}

impl AssetEditorDrawContext {
    pub fn mark_as_unsaved(&mut self) {
        self.need_save = true;
    }
}

impl AssetEditorManager {
    pub fn new(filesystem: Arc<FileSystem>, asset_server: Arc<AssetServer>) -> Self {
        Self {
            filesystem,
            asset_server,
            editor_factories: Default::default(),
            editors: Default::default(),
        }
    }

    pub fn add_editor_factory<F: AssetEditorFactory + 'static>(&self, type_uuid: Uuid, editor: F) {
        let mut editors = self.editor_factories.lock();
        editors.insert(type_uuid, Box::new(editor));
    }

    pub fn open_asset(&self, type_uuid: Uuid, uuid: Uuid, source_path: &Path) {
        let mut editors = self.editors.lock();
        for entry in editors.iter() {
            if entry.editor.asset_uuid() == uuid {
                return;
            }
        }

        let factories = self.editor_factories.lock();
        if let Some(factory) = factories.get(&type_uuid) {
            let metadata_path = {
                let mut path = source_path.clone();
                let asset_path =
                    path.path().to_string().rsplit('.').collect::<Vec<&str>>()[1].to_string();
                let path_str = format!("{}.{}", asset_path, ASSET_METADATA_EXTENSION);
                path.set_path(&path_str);
                path
            };

            // TODO: Manage errors
            if let Some(editor) = factory.open(&self.filesystem, uuid, source_path, &metadata_path)
            {
                editors.push(AssetEditorEntry::new(editor, source_path.clone()));
            }
        }
    }

    pub fn draw_editors(&self, imgui: &mut Context, main_dockspace_id: ImGuiID) {
        let mut editors = self.editors.lock();
        editors.retain(|entry| !entry.closed);

        for entry in editors.iter_mut() {
            imgui.next_window_dock_id(main_dockspace_id);
            let mut is_open = true;
            let mut flags = WindowFlags::from_flag(WindowFlagBits::NoSavedSettings);
            if entry.unsaved {
                flags.insert(WindowFlagBits::UnsavedDocument);
            }

            if imgui.begin_window_closable(entry.source_path.path(), &mut is_open, flags) {
                let mut context = AssetEditorDrawContext::default();
                entry.editor.draw(imgui, &mut context);

                if imgui.is_key_down(Key::LeftCtrl) && imgui.is_key_pressed(Key::S, false) {
                    ze_info!("Saving {}", entry.source_path);
                    if entry.editor.save(&self.filesystem) {
                        ze_info!("Saved {}", entry.source_path);
                        entry.unsaved = false;
                        self.asset_server.import_source_asset(&entry.source_path);
                    }
                } else if context.need_save {
                    entry.unsaved = true;
                }
            }

            imgui.set_next_window_pos(
                imgui.main_viewport().center(),
                Cond::Appearing,
                ImVec2::new(0.5, 0.5),
            );
            if imgui.begin_popup_modal(
                "##Close",
                &mut entry.closing,
                make_bitflags! { WindowFlagBits::{AlwaysAutoResize} },
            ) {
                imgui.dummy(ImVec2::new(1.0, 15.0));
                imgui
                    .text_centered_wrapped("Asset not saved, are you sure you want to close?", 100);
                imgui.dummy(ImVec2::new(1.0, 25.0));
                if imgui.button("Save", ImVec2::new(120.0, 0.0))
                    && entry.editor.save(&self.filesystem)
                {
                    imgui.close_current_popup();
                    self.asset_server.import_source_asset(&entry.source_path);
                    entry.closed = true;
                }

                imgui.same_line(0.0, -1.0);
                if imgui.button("Don't save", ImVec2::new(120.0, 0.0)) {
                    imgui.close_current_popup();
                    entry.closed = true;
                }
                imgui.same_line(0.0, -1.0);
                if imgui.button("Cancel", ImVec2::new(120.0, 0.0)) {
                    imgui.close_current_popup();
                }
                imgui.dummy(ImVec2::new(1.0, 15.0));
                imgui.end_popup();
            }

            if !is_open {
                if entry.unsaved {
                    entry.closing = true;
                    imgui.open_popup("##Close");
                } else {
                    entry.closed = true;
                }
            }

            imgui.end_window();
        }
    }
}
