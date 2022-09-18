use harfbuzz_rs::Owned;
use parking_lot::Mutex;
use serde::{Deserialize, Deserializer, Serialize};
use std::sync::Arc;
use ze_asset_system::Asset;
use ze_core::type_uuid::*;

#[derive(Serialize, Deserialize, Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub enum FontWeight {
    Thin = 100,
    ExtraLight = 200,
    Light = 300,
    Regular = 400,
    Medium = 500,
    SemiBold = 600,
    Bold = 700,
    ExtraBold = 800,
    Black = 900,
}

impl Default for FontWeight {
    fn default() -> Self {
        Self::Regular
    }
}

#[derive(Serialize, Deserialize, Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub enum FontStyle {
    Normal,
    Italic,
}

impl Default for FontStyle {
    fn default() -> Self {
        Self::Normal
    }
}

/// A family of font faces
#[derive(Serialize, Deserialize, TypeUuid)]
#[type_uuid = "daf7e182-d479-4370-a7e2-240e9153fe79"]
pub struct FontFamily {
    #[serde(skip_serializing, skip_deserializing)]
    uuid: Uuid,
    faces: Vec<(FontWeight, FontStyle, Uuid)>,
}

impl FontFamily {
    pub fn new() -> Self {
        Self {
            uuid: Uuid::new_v4(),
            faces: Vec::new(),
        }
    }

    pub fn face(&self, weight: FontWeight, style: FontStyle) -> Option<Uuid> {
        for (w, s, uuid) in &self.faces {
            if *w == weight && *s == style {
                return Some(*uuid);
            }
        }

        None
    }
}

impl Asset for FontFamily {
    fn uuid(&self) -> Uuid {
        self.uuid
    }
}

/// Store specific typeface glyphs configuration (weight, style)
#[derive(Serialize, Deserialize, TypeUuid)]
#[type_uuid = "aef49b50-2202-4da7-a06a-eef11023fa01"]
pub struct FontFace {
    #[serde(skip_serializing, skip_deserializing)]
    uuid: Uuid,
    data: Box<dyn FontFaceData>,
}

impl FontFace {
    pub fn new(uuid: Uuid, data: Box<dyn FontFaceData>) -> Self {
        Self { uuid, data }
    }

    pub fn data(&self) -> &dyn FontFaceData {
        &*self.data
    }
}

impl Asset for FontFace {
    fn uuid(&self) -> Uuid {
        self.uuid
    }
}

pub enum GlyphBitmapData {
    R8DistanceField(Vec<u8>),
}

pub struct GlyphBitmap {
    pub width: u32,
    pub height: u32,
    pub pitch: u32,
    pub data: GlyphBitmapData,
}

impl GlyphBitmap {
    pub fn new(width: u32, height: u32, pitch: u32, data: GlyphBitmapData) -> Self {
        Self {
            width,
            height,
            pitch,
            data,
        }
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    pub fn data(&self) -> &GlyphBitmapData {
        &self.data
    }
}

/// Object containing font face data, can be TTF, or bitmap, ...
/// It abstracts how to render it and how to shape it
#[typetag::serde(tag = "FontFaceData")]
pub trait FontFaceData: Send + Sync {
    /// Returns the bitmap for this glyphmap
    fn glyph_bitmap(&self, glyph_index: u32) -> Option<GlyphBitmap>;
    fn shape(&self);
}

/// Font data for FreeType-compatible fonts (TTF, OTF)
/// Glyph rendering is done by FreeType
/// Shaping is done by HarfBuzz
#[derive(serde_derive::Serialize)]
pub struct FreeTypeFontData {
    data: Vec<u8>,

    #[serde(skip_serializing)]
    _ft_face: Mutex<ze_freetype::Face>,

    #[serde(skip_serializing)]
    _hb_face: Owned<harfbuzz_rs::Face<'static>>,

    #[serde(skip_serializing)]
    _ft_library: Arc<Mutex<ze_freetype::Library>>,
}

impl FreeTypeFontData {
    pub fn new(
        ft_library: Arc<Mutex<ze_freetype::Library>>,
        data: Vec<u8>,
    ) -> Result<Self, ze_freetype::Error> {
        let ft_face = {
            let mut library = ft_library.lock();
            library.new_memory_face(&data, 0)?
        };

        // FIXME: Find a better way instead of cloning
        let hb_face = harfbuzz_rs::Face::new(data.clone(), 0);
        Ok(Self {
            _ft_library: ft_library,
            data,
            _ft_face: Mutex::new(ft_face),
            _hb_face: hb_face,
        })
    }
}

#[typetag::serde]
impl FontFaceData for FreeTypeFontData {
    fn glyph_bitmap(&self, _: u32) -> Option<GlyphBitmap> {
        todo!()
    }

    fn shape(&self) {
        todo!()
    }
}

impl<'de> Deserialize<'de> for FreeTypeFontData {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let _: Vec<u8> = Deserialize::deserialize(deserializer)?;

        todo!()
    }
}
