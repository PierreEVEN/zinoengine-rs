use crate::icon_manager::{Icon, IconManager};
use enumflags2::make_bitflags;
use std::cmp::Ordering;
use std::str::FromStr;
use std::sync::Arc;
use ze_asset_editor::AssetEditorManager;
use ze_asset_server::AssetServer;
use ze_filesystem::path::Path;
use ze_filesystem::{DirEntryType, FileSystem, IterDirFlags};
use ze_imgui::ze_imgui_sys::*;
use ze_imgui::*;
use ze_platform::MouseButton;

pub const ASSET_EXPLORER_ID: &str = "Asset Explorer";

pub struct AssetExplorer {
    asset_server: Arc<AssetServer>,
    filesystem: Arc<FileSystem>,
    current_directory: Path,
    directory_icon: Option<Arc<Icon>>,
    file_icon: Option<Arc<Icon>>,
    asset_editor_manager: Arc<AssetEditorManager>,
}

impl AssetExplorer {
    pub fn new(
        asset_server: Arc<AssetServer>,
        icon_manager: Arc<IconManager>,
        filesystem: Arc<FileSystem>,
        asset_editor_manager: Arc<AssetEditorManager>,
    ) -> Self {
        Self {
            asset_server,
            filesystem,
            current_directory: Path::parse("/main/assets").unwrap(),
            directory_icon: icon_manager.icon("icons8-folder-64"),
            file_icon: icon_manager.icon("icons8-file-64"),
            asset_editor_manager,
        }
    }

    pub fn draw(&mut self, imgui: &mut Context) {
        puffin::profile_function!();
        imgui.begin_window(
            ASSET_EXPLORER_ID,
            make_bitflags! { WindowFlagBits::{NoScrollbar | NoScrollWithMouse}},
        );

        if imgui.begin_table(
            "MainTable",
            2,
            make_bitflags! { TableFlagBits::{Resizable | NoBordersInBodyUntilResize} },
            imgui.available_content_region(),
        ) {
            imgui.table_setup_column(
                "Directory Hierarchy",
                0.1,
                TableColumnFlags::from_flag(TableColumnFlagBits::WidthStretch),
            );
            imgui.table_next_row();
            imgui.table_next_column();

            // Directory list
            self.draw_directory_hierarchy(imgui);

            imgui.table_next_column();
            self.draw_directory_list(imgui);
            imgui.end_table();
        }

        imgui.end_window();
    }

    fn draw_directory_list(&mut self, imgui: &mut Context) {
        puffin::profile_function!();
        imgui.begin_child(
            "Directory List",
            imgui.available_content_region(),
            false,
            WindowFlags::empty(),
        );

        imgui.dummy(ImVec2::new(0.0, 5.0));
        imgui.dummy(ImVec2::new(10.0, 0.0));
        imgui.same_line(0.0, -1.0);

        let column_count = (imgui.available_content_region().x / 90.0).clamp(1.0, 15.0);
        imgui.begin_table(
            "DirectoryListTable",
            column_count as u32,
            TableFlags::empty(),
            ImVec2::default(),
        );

        imgui.table_next_row();

        let mut entries = vec![];
        self.filesystem
            .iter_dir(&self.current_directory, IterDirFlags::empty(), |entry| {
                if entry.ty == DirEntryType::File {
                    let file_name_and_extension =
                        entry.path.path_segments().last().unwrap().split('.');

                    if !self
                        .asset_server
                        .is_extension_importable(file_name_and_extension.last().unwrap())
                    {
                        return;
                    }
                }
                entries.push(entry.clone());
            })
            .unwrap();

        entries.sort_by(|a, b| {
            if a.ty != b.ty && a.ty == DirEntryType::Directory {
                Ordering::Less
            } else {
                Ordering::Greater
            }
        });

        for entry in entries {
            let mut file_name_and_extension = entry.path.path_segments().last().unwrap().split('.');

            let file_name = file_name_and_extension.next().unwrap();

            imgui.table_next_column();

            imgui.begin_child(
                entry.path.as_str(),
                ImVec2::new(90.0, 150.0),
                false,
                make_bitflags! { WindowFlagBits::{NoScrollbar | NoScrollWithMouse} },
            );

            let cursor_screen_pos = imgui.cursor_screen_pos();
            imgui.window_add_rect_filled(
                cursor_screen_pos,
                cursor_screen_pos + imgui.available_content_region(),
                ImVec4::from(0.115),
            );

            if imgui.is_window_hovered() {
                if imgui.is_mouse_double_clicked(MouseButton::Left) {
                    if entry.ty == DirEntryType::Directory {
                        self.current_directory = entry.path.clone();
                    } else if let Some(uuid) = self.asset_server.asset_uuid_from_path(&entry.path) {
                        if let Some(type_uuid) = self.asset_server.asset_type_uuid(uuid) {
                            self.asset_editor_manager
                                .open_asset(type_uuid, uuid, &entry.path);
                        }
                    }
                }

                if entry.ty == DirEntryType::File {
                    imgui.push_style_var_vec2f32(StyleVar::WindowPadding, ImVec2::new(5.0, 5.0));
                    imgui.begin_tooltip();
                    imgui.pop_style_var(1);
                    imgui.text(entry.path.path());
                    imgui.end_tooltip();
                }

                let cursor_screen_pos = imgui.cursor_screen_pos();
                imgui.window_add_rect_filled(
                    cursor_screen_pos,
                    cursor_screen_pos + imgui.available_content_region(),
                    unsafe { (*igGetStyle()).Colors[ImGuiCol__ImGuiCol_HeaderHovered as usize] },
                );
            }

            imgui.dummy(ImVec2::new(0.0, 15.0));

            let icon = if entry.ty == DirEntryType::Directory {
                &self.directory_icon
            } else {
                &self.file_icon
            };
            if let Some(icon) = icon {
                imgui.image_centered_x(&icon.srv, ImVec2::new(64.0, 64.0));
            }

            imgui.dummy(ImVec2::new(0.0, 5.0));
            imgui.text_centered_wrapped(file_name, 8);
            imgui.end_child();
        }

        imgui.end_table();
        imgui.end_child();
    }

    fn draw_directory_hierarchy(&mut self, imgui: &mut Context) {
        // TODO: Cache hierarchy

        puffin::profile_function!();
        imgui.begin_child(
            "Directory Hierarchy",
            imgui.available_content_region(),
            false,
            WindowFlags::empty(),
        );
        self.draw_directory_entry(imgui, &Path::parse("/main/assets").unwrap());
        imgui.end_child();
    }

    fn draw_directory_entry(&mut self, imgui: &mut Context, path: &Path) {
        puffin::profile_function!(path.path());

        let mut flags = make_bitflags! { TreeNodeFlagBits::{DefaultOpen | OpenOnArrow | OpenOnDoubleClick | SpanFullWidth } };
        if self.current_directory == *path {
            flags.insert(TreeNodeFlagBits::Selected);
        }

        if imgui.tree_node_ex(
            &format!("##{}", path.path_segments().last().unwrap()),
            flags,
        ) {
            if imgui.is_item_clicked(MouseButton::Left) {
                self.current_directory = path.clone();
            }

            imgui.same_line(0.0, -1.0);
            if let Some(icon) = &self.directory_icon {
                imgui.image(&icon.srv, ImVec2::new(16.0, 16.0));
                imgui.same_line(0.0, -1.0);
            }
            imgui.text(path.path_segments().last().unwrap());

            let filesystem = self.filesystem.clone();
            filesystem
                .iter_dir(path, IterDirFlags::empty(), |entry| {
                    if entry.ty == DirEntryType::Directory {
                        self.draw_directory_entry(imgui, &entry.path);
                    }
                })
                .unwrap();
            imgui.tree_pop();
        }
    }
}
