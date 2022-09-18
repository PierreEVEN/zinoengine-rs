use std::collections::HashMap;
use std::str::FromStr;
use std::sync::{Arc, Weak};
use ze_asset_system::url::Url;
use ze_asset_system::AssetManager;
use ze_core::type_uuid::Uuid;
use ze_font_asset::{FontFamily, FontStyle, FontWeight};

pub struct Font {
    family: Uuid,
    weight: FontWeight,
    style: FontStyle,
    size: i32,
}

impl Font {
    pub fn new(family: Uuid, weight: FontWeight, style: FontStyle, size: i32) -> Self {
        Self {
            family,
            weight,
            style,
            size,
        }
    }

    pub fn family(&self) -> &Uuid {
        &self.family
    }

    pub fn weight(&self) -> FontWeight {
        self.weight
    }

    pub fn style(&self) -> FontStyle {
        self.style
    }

    pub fn size(&self) -> i32 {
        self.size
    }
}

/// Object responsible for caching font families
/// Primarily used to not make useless requests to the asset manager
pub struct FontCache {
    asset_manager: Arc<AssetManager>,
    font_families: HashMap<Uuid, Weak<FontFamily>>,
}

impl FontCache {
    pub fn new(asset_manager: Arc<AssetManager>) -> Self {
        Self {
            asset_manager,
            font_families: Default::default(),
        }
    }

    pub fn font_family(&mut self, uuid: &Uuid) -> Option<Arc<FontFamily>> {
        if let Some(family) = self.font_families.get(&uuid) {
            if let Some(family) = family.upgrade() {
                return Some(family);
            }
        }

        if let Ok(_family) = self
            .asset_manager
            .load_sync(&Url::from_str(&format!("asset:///{}", uuid.to_string())).unwrap())
        {
            None
        } else {
            None
        }
    }
}
