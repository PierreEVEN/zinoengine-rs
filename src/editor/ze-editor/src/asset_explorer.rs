use crate::icon_manager::{Icon, IconManager};
use enumflags2::make_bitflags;
use std::cmp::Ordering;
use std::str::FromStr;
use std::sync::Arc;
use url::Url;
use ze_asset_server::AssetServer;
use ze_filesystem::{DirEntryType, FileSystem, IterDirFlags};
use ze_imgui::ze_imgui_sys::*;
use ze_imgui::*;
use ze_platform::MouseButton;

pub const ASSET_EXPLORER_ID: &str = "Asset Explorer";

pub struct AssetExplorer {
    asset_server: Arc<AssetServer>,
    filesystem: Arc<FileSystem>,
    current_directory: Url,
    directory_icon: Option<Arc<Icon>>,
    file_icon: Option<Arc<Icon>>,
}

impl AssetExplorer {
    pub fn new(
        asset_server: Arc<AssetServer>,
        icon_manager: Arc<IconManager>,
        filesystem: Arc<FileSystem>,
    ) -> Self {
        Self {
            asset_server,
            filesystem,
            current_directory: Url::from_str("vfs://main/assets").unwrap(),
            directory_icon: icon_manager.icon("icons8-folder-64"),
            file_icon: icon_manager.icon("icons8-file-64"),
        }
    }

    pub fn draw(&mut self, imgui: &mut Context) {
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
        imgui.begin_child(
            "Directory List",
            imgui.available_content_region(),
            false,
            WindowFlags::empty(),
        );

        imgui.dummy(ImVec2::new(0.0, 10.0));
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
                    let file_name_and_extension = entry
                        .url
                        .path_segments()
                        .unwrap()
                        .last()
                        .unwrap()
                        .split('.');

                    if !self
                        .asset_server
                        .is_extension_importable(file_name_and_extension.last().unwrap())
                    {
                        return;
                    }
                }
                entries.push(entry);
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
            let mut file_name_and_extension = entry
                .url
                .path_segments()
                .unwrap()
                .last()
                .unwrap()
                .split('.');

            let file_name = file_name_and_extension.next().unwrap();
            let extension = file_name_and_extension.next();

            imgui.table_next_column();

            imgui.begin_child(
                entry.url.as_str(),
                ImVec2::new(90.0, 120.0),
                false,
                WindowFlags::empty(),
            );
            if imgui.is_window_hovered() {
                if entry.ty == DirEntryType::Directory
                    && imgui.is_mouse_double_clicked(MouseButton::Left)
                {
                    self.current_directory = entry.url.clone();
                }

                let cursor_screen_pos = imgui.cursor_screen_pos();
                imgui.window_add_rect_filled(
                    cursor_screen_pos,
                    cursor_screen_pos + imgui.available_content_region(),
                    unsafe { (*igGetStyle()).Colors[ImGuiCol__ImGuiCol_HeaderHovered as usize] },
                );

                if entry.ty == DirEntryType::File {
                    imgui.push_style_var_Vec2f32(StyleVar::WindowPadding, ImVec2::new(5.0, 5.0));
                    imgui.begin_tooltip();
                    imgui.pop_style_var(1);

                    imgui.text(&format!("Format: {}", extension.unwrap()));

                    imgui.end_tooltip();
                }
            }

            imgui.dummy(ImVec2::new(0.0, 5.0));

            let icon = if entry.ty == DirEntryType::Directory {
                &self.directory_icon
            } else {
                &self.file_icon
            };
            if let Some(icon) = icon {
                imgui.image_centered(&icon.srv, ImVec2::new(64.0, 64.0));
            }

            imgui.text_centered_wrapped(file_name, 8);
            imgui.end_child();
        }

        imgui.end_table();
        imgui.end_child();
    }

    fn draw_directory_hierarchy(&mut self, imgui: &mut Context) {
        imgui.begin_child(
            "Directory Hierarchy",
            imgui.available_content_region(),
            false,
            WindowFlags::empty(),
        );
        self.draw_hierarchy_recursive(imgui, &Url::from_str("vfs://main/assets").unwrap());
        imgui.end_child();
    }

    fn draw_hierarchy_recursive(&mut self, imgui: &mut Context, url: &Url) {
        let mut flags = make_bitflags! { TreeNodeFlagBits::{DefaultOpen | OpenOnArrow | OpenOnDoubleClick | SpanFullWidth } };
        if self.current_directory == *url {
            flags.insert(TreeNodeFlagBits::Selected);
        }

        if imgui.tree_node_ex(
            &format!("##{}", url.path_segments().unwrap().last().unwrap()),
            flags,
        ) {
            if imgui.is_item_clicked(MouseButton::Left) {
                self.current_directory = url.clone();
            }

            imgui.same_line(0.0, -1.0);
            if let Some(icon) = &self.directory_icon {
                imgui.image(&icon.srv, ImVec2::new(16.0, 16.0));
                imgui.same_line(0.0, -1.0);
            }
            imgui.text(url.path_segments().unwrap().last().unwrap());

            let filesystem = self.filesystem.clone();
            filesystem
                .iter_dir(url, IterDirFlags::empty(), |entry| {
                    if entry.ty == DirEntryType::Directory {
                        self.draw_hierarchy_recursive(imgui, &entry.url);
                    }
                })
                .unwrap();
            imgui.tree_pop();
        }
    }
}
