use ze_imgui::Context;

pub const ASSET_EXPLORER_ID: &str = "Asset Explorer";

#[derive(Default)]
pub struct AssetExplorer {}

impl AssetExplorer {
    pub fn draw(&self, imgui: &mut Context) {
        imgui.window(ASSET_EXPLORER_ID).begin();
        imgui.end_window();
    }
}
