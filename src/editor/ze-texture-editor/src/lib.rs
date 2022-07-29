use std::str::FromStr;
use std::sync::Arc;
use url::Url;
use uuid::Uuid;
use ze_asset_editor::{AssetEditor, AssetEditorFactory};
use ze_asset_system::{Asset, AssetManager};
use ze_imgui::ze_imgui_sys::ImVec2;
use ze_imgui::{WindowFlagBits, WindowFlags};
use ze_texture_asset::Texture;

pub struct Editor {
    uuid: Uuid,
    texture: Arc<dyn Asset>,
}

impl AssetEditor for Editor {
    fn draw(&self, imgui: &mut ze_imgui::Context) {
        let texture = self.texture.downcast_ref::<Texture>().unwrap();

        imgui.begin_window(
            &self.uuid.to_string(),
            WindowFlags::from_flag(WindowFlagBits::NoSavedSettings),
        );
        imgui.text(&format!("Texture Editor for {}", self.uuid));

        if let Some(default_srv) = texture.default_srv() {
            imgui.image(&default_srv, ImVec2::new(256.0, 256.0));
        }

        imgui.end_window();
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
        })
    }
}
