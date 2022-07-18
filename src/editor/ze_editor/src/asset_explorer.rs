use enumflags2::make_bitflags;
use std::str::FromStr;
use std::sync::Arc;
use url::{Position, Url};
use ze_filesystem::{DirEntryType, FileSystem, IterDirFlagBits, IterDirFlags};
use ze_imgui::ze_imgui_sys::ImVec2;
use ze_imgui::*;
use ze_platform::MouseButton;

pub const ASSET_EXPLORER_ID: &str = "Asset Explorer";

pub struct AssetExplorer {
    filesystem: Arc<FileSystem>,
    current_directory: Url,
}

impl AssetExplorer {
    pub fn new(filesystem: Arc<FileSystem>) -> Self {
        Self {
            filesystem,
            current_directory: Url::from_str("vfs://main/assets").unwrap(),
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
            make_bitflags! { TableFlagBits::{Resizable | BordersInnerV} },
            imgui.get_available_content_region(),
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

    fn draw_directory_list(&self, imgui: &mut Context) {
        imgui.begin_child(
            "Directory List",
            imgui.get_available_content_region(),
            false,
        );

        let column_count = (imgui.get_available_content_region().x / 90.0)
            .min(15.0)
            .max(0.0);
        imgui.begin_table(
            "DirectoryListTable",
            column_count as u32,
            TableFlags::empty(),
            ImVec2::default(),
        );

        imgui.table_next_row();

        self.filesystem
            .iter_dir(&self.current_directory, IterDirFlags::empty(), |entry| {
                imgui.table_next_column();

                imgui.begin_child(entry.url.as_str(), ImVec2::new(90.0, 120.0), false);
                imgui.text_wrapped(entry.url.path_segments().unwrap().last().unwrap());
                imgui.end_child();
            })
            .unwrap();

        imgui.end_table();
        imgui.end_child();
    }

    fn draw_directory_hierarchy(&mut self, imgui: &mut Context) {
        imgui.begin_child(
            "Directory Hierarchy",
            imgui.get_available_content_region(),
            false,
        );
        self.draw_hierarchy_recursive(imgui, &Url::from_str("vfs://main/assets").unwrap());
        imgui.end_child();
    }

    fn draw_hierarchy_recursive(&mut self, imgui: &mut Context, url: &Url) {
        let mut flags = make_bitflags! { TreeNodeFlagBits::{DefaultOpen | OpenOnArrow | OpenOnDoubleClick | SpanFullWidth } };
        if self.current_directory == *url {
            flags.insert(TreeNodeFlagBits::Selected);
        }

        if imgui.tree_node_ex(url.path_segments().unwrap().last().unwrap(), flags) {
            if imgui.is_item_clicked(MouseButton::Left) {
                self.current_directory = url.clone();
            }

            let filesystem = self.filesystem.clone();
            filesystem
                .iter_dir(
                    url,
                    IterDirFlags::from_flag(IterDirFlagBits::Recursive),
                    |entry| {
                        if entry.ty == DirEntryType::Directory {
                            self.draw_hierarchy_recursive(imgui, &entry.url);
                        }
                    },
                )
                .unwrap();
            imgui.tree_pop();
        }
    }
}
