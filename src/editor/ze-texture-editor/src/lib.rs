use enumflags2::make_bitflags;
use std::str::FromStr;
use std::sync::Arc;
use url::Url;
use uuid::Uuid;
use ze_asset_editor::{AssetEditor, AssetEditorFactory};
use ze_asset_system::{Asset, AssetManager};
use ze_imgui::ze_imgui_sys::{ImVec2, ImVec4};
use ze_imgui::{
    Context, TableColumnFlagBits, TableColumnFlags, TableFlagBits, WindowFlagBits, WindowFlags,
};
use ze_property_editor::draw_property_editor;
use ze_reflection::*;
use ze_texture_asset::Texture;

pub struct Editor {
    uuid: Uuid,
    texture: Arc<dyn Asset>,
    import_parameters: ze_texture_asset::importer::Parameters,
}

impl Editor {
    fn draw_properties(&mut self, imgui: &mut Context) {
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
            "Size in VRAM (mip 0): {} Kb",
            (texture_size as f32 * 0.001).trunc()
        ));
        imgui.text(&format!("Format: {}", texture.format()));
        imgui.text(&format!("Mip levels: {}", texture.mip_levels().len()));

        imgui.separator();
        imgui.text("Importer Parameters");
        imgui.dummy(ImVec2::new(0.0, 3.0));

        let parameters_type_desc = ze_texture_asset::importer::Parameters::type_desc();
        let _parameters_type_desc_data: &StructDescription = parameters_type_desc.data_as_struct();

        /*
        imgui.begin_table(
            "PropertiesTable",
            2,
            make_bitflags! { TableFlagBits::{Resizable | NoBordersInBodyUntilResize} },
            imgui.available_content_region(),
        );

        imgui.table_setup_column(
            "Name",
            0.25,
            TableColumnFlags::from_flag(TableColumnFlagBits::WidthStretch),
        );

        imgui.table_setup_column(
            "Value",
            0.75,
            TableColumnFlags::from_flag(TableColumnFlagBits::WidthStretch),
        );
        */

        //imgui.table_next_row();
        //imgui.table_next_column();
        draw_property_editor(imgui, &mut self.import_parameters);

        //imgui.end_table();
    }
}

impl AssetEditor for Editor {
    fn draw(&mut self, imgui: &mut Context) {
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
                self.draw_properties(imgui);
                imgui.end_child();
            }

            imgui.end_table();
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
    fn open(&self, asset: Uuid) -> Box<dyn AssetEditor> {
        let texture = self
            .asset_manager
            .load(&Url::from_str(&format!("assets:///{}", asset)).unwrap())
            .unwrap();

        Box::new(Editor {
            uuid: asset,
            texture,
            import_parameters: ze_texture_asset::importer::Parameters::default(),
        })
    }
}
