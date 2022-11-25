use enumflags2::make_bitflags;
use std::sync::Arc;
use uuid::Uuid;
use ze_asset_editor::{AssetEditor, AssetEditorDrawContext, AssetEditorFactory};
use ze_asset_system::importer::SourceAssetMetadata;
use ze_asset_system::{Asset, AssetManager};
use ze_filesystem::path::Path;
use ze_filesystem::FileSystem;
use ze_imgui::ze_imgui_sys::{ImVec2, ImVec4};
use ze_imgui::{
    Context, TableColumnFlagBits, TableColumnFlags, TableFlagBits, WindowFlagBits, WindowFlags,
};
use ze_property_editor::draw_property_editor;
use ze_texture_asset::Texture;

pub struct Editor {
    uuid: Uuid,
    _source_path: Path,
    metadata_path: Path,
    texture: Arc<dyn Asset>,
    metadata: SourceAssetMetadata<(), ze_texture_asset::importer::Parameters>,
}

impl Editor {
    fn draw_properties(&mut self, imgui: &mut Context) -> bool {
        let texture = self.texture.downcast_ref::<Texture>().unwrap();

        imgui.text(&format!(
            "Dimensions: {}x{}x{}",
            texture.width(),
            texture.height(),
            texture.depth(),
        ));

        let texture_size = texture
            .format()
            .texture_size_in_bytes(texture.width(), texture.height());

        imgui.text(&format!(
            "Size in VRAM (mip 0): {:.3} Mb",
            texture_size as f32 / 1e+6
        ));
        imgui.text(&format!("Format: {}", texture.format()));
        imgui.text(&format!("Mip levels: {}", texture.mip_levels().len()));

        imgui.separator();
        imgui.text("Importer Parameters");
        imgui.dummy(ImVec2::new(0.0, 3.0));

        draw_property_editor(imgui, self.metadata.parameters_mut())
    }
}

impl AssetEditor for Editor {
    fn draw(&mut self, imgui: &mut Context, context: &mut AssetEditorDrawContext) {
        let texture = self.texture.downcast_ref::<Texture>().unwrap();
        if imgui.begin_table(
            "MainTable",
            2,
            make_bitflags! { TableFlagBits::{Resizable} },
            imgui.available_content_region(),
        ) {
            imgui.table_setup_column(
                "Preview",
                0.7,
                TableColumnFlags::from_flag(TableColumnFlagBits::WidthStretch),
            );

            imgui.table_setup_column(
                "Details",
                0.3,
                TableColumnFlags::from_flag(TableColumnFlagBits::WidthStretch),
            );

            imgui.table_next_row();
            imgui.table_next_column();

            // Texture Preview
            if imgui.begin_child(
                "TexturePreview",
                imgui.available_content_region(),
                false,
                WindowFlags::empty(),
            ) {
                // Black background
                {
                    let cursor_screen_pos = imgui.cursor_screen_pos();
                    imgui.window_add_rect_filled(
                        cursor_screen_pos,
                        cursor_screen_pos + imgui.available_content_region(),
                        ImVec4::from(0.0),
                    );
                }

                if let Some(default_srv) = texture.default_srv() {
                    imgui.image_centered(
                        &default_srv,
                        ImVec2::new(texture.width() as f32, texture.height() as f32),
                    );
                }
                imgui.end_child();
            }

            // Properties Editor
            imgui.table_next_column();
            if imgui.begin_child(
                "Properties",
                imgui.available_content_region(),
                false,
                make_bitflags!(WindowFlagBits::{AlwaysUseWindowPadding}),
            ) {
                if self.draw_properties(imgui) {
                    context.mark_as_unsaved();
                }
                imgui.end_child();
            }

            imgui.end_table();
        }
    }

    fn save(&self, filesystem: &FileSystem) -> bool {
        let yaml = match serde_yaml::to_string(&self.metadata) {
            Ok(str) => str,
            Err(_) => return false,
        };

        if let Ok(mut metadata_file) = filesystem.write(&self.metadata_path) {
            metadata_file.write_all(yaml.as_bytes()).is_ok()
        } else {
            false
        }
    }

    fn asset_uuid(&self) -> Uuid {
        self.uuid
    }
}

pub struct EditorFactory {
    asset_manager: Arc<AssetManager>,
}

impl EditorFactory {
    pub fn new(asset_manager: Arc<AssetManager>) -> Self {
        Self { asset_manager }
    }
}

impl AssetEditorFactory for EditorFactory {
    fn open(
        &self,
        filesystem: &FileSystem,
        asset: Uuid,
        source_path: &Path,
        metadata_path: &Path,
    ) -> Option<Box<dyn AssetEditor>> {
        let texture = self
            .asset_manager
            .load_sync(&Path::parse(&format!("//{}", asset)).unwrap())
            .unwrap();

        if let Ok(metadata_buf) = filesystem.read(metadata_path) {
            if let Ok(metadata) = metadata_buf.try_into() {
                Some(Box::new(Editor {
                    uuid: asset,
                    _source_path: source_path.clone(),
                    metadata_path: metadata_path.clone(),
                    texture,
                    metadata,
                }))
            } else {
                None
            }
        } else {
            None
        }
    }
}
