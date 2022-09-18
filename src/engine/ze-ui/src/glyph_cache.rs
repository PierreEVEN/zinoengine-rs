use rectangle_pack::{
    contains_smallest_box, pack_rects, volume_heuristic, GroupedRectsToPlace, PackedLocation,
    RectToInsert, RectanglePackError, TargetBin,
};
use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;
use ze_asset_system::Asset;
use ze_core::maths::Vec2u32;
use ze_core::type_uuid::Uuid;
use ze_font_asset::{FontFace, GlyphBitmapData};
use ze_gfx::backend::{
    Device, MemoryLocation, ResourceState, Texture, TextureDesc, TextureUsageFlags,
};
use ze_gfx::{utils, PixelFormat};

/// Must be a ID unique for each font
pub type BinId = u64;

pub struct PositionedGlyph {
    _font_face: Uuid,
    _bin: BinId,
    _glyph: u32,
    _position: Vec2u32,
}

/// Object caching font characters into textures for rendering purposes
pub struct Cache {
    device: Arc<dyn Device>,
    atlases_size: Vec2u32,
    font_face_caches: HashMap<Uuid, FontFaceGlyphCache>,
}

impl Cache {
    pub fn new(device: Arc<dyn Device>, atlases_size: Vec2u32) -> Self {
        Self {
            device,
            atlases_size,
            font_face_caches: Default::default(),
        }
    }

    pub fn add_font_face(&mut self, font_face: Arc<FontFace>) {
        self.font_face_caches.insert(
            font_face.uuid(),
            FontFaceGlyphCache::new(self.device.clone(), font_face, self.atlases_size),
        );
    }

    pub fn insert_glyph(&mut self, font_face: &Arc<FontFace>, glyph_index: u32) -> bool {
        if let Some(cache) = self.font_face_caches.get_mut(&font_face.uuid()) {
            cache.insert(glyph_index);
            true
        } else {
            false
        }
    }

    pub fn flush_glyph_queue(&mut self, font_face: &Arc<FontFace>) {
        if let Some(cache) = self.font_face_caches.get_mut(&font_face.uuid()) {
            while !cache.pack() {
                cache.add_bin();
            }

            cache.render();
        }
    }

    pub fn glyph(&self, font_face_uuid: Uuid, glyph_index: u32) -> Option<PositionedGlyph> {
        if let Some(cache) = self.font_face_caches.get(&font_face_uuid) {
            cache
                .glyph_location(glyph_index)
                .map(|location| PositionedGlyph {
                    _font_face: font_face_uuid,
                    _bin: location.0,
                    _glyph: glyph_index,
                    _position: Vec2u32::new(location.1.x(), location.1.y()),
                })
        } else {
            None
        }
    }
}

struct FontFaceGlyphCache {
    device: Arc<dyn Device>,
    font_face: Arc<FontFace>,
    atlases_size: Vec2u32,
    queued_glyph_rects: GroupedRectsToPlace<u32>,
    glyph_locations: BTreeMap<u32, (BinId, PackedLocation)>,
    bins: BTreeMap<BinId, TargetBin>,
    bin_textures: HashMap<BinId, Texture>,
    bin_buffers: HashMap<BinId, Vec<u8>>,
    free_bin_id: BinId,
}

impl FontFaceGlyphCache {
    pub fn new(device: Arc<dyn Device>, font_face: Arc<FontFace>, atlases_size: Vec2u32) -> Self {
        Self {
            device,
            font_face,
            atlases_size,
            queued_glyph_rects: GroupedRectsToPlace::new(),
            bins: Default::default(),
            glyph_locations: BTreeMap::new(),
            bin_textures: Default::default(),
            bin_buffers: Default::default(),
            free_bin_id: 0,
        }
    }

    fn glyph_location(&self, glyph_index: u32) -> Option<&(BinId, PackedLocation)> {
        self.glyph_locations.get(&glyph_index)
    }

    fn insert(&mut self, glyph_index: u32) {
        let bitmap = match self.font_face.data().glyph_bitmap(glyph_index) {
            Some(glyph) => glyph,
            None => return,
        };

        self.queued_glyph_rects.push_rect(
            glyph_index,
            None,
            RectToInsert::new(bitmap.width(), bitmap.height(), 1),
        );
    }

    fn pack(&mut self) -> bool {
        match pack_rects(
            &self.queued_glyph_rects,
            &mut self.bins,
            &volume_heuristic,
            &contains_smallest_box,
        ) {
            Ok(ok) => {
                self.glyph_locations = ok.packed_locations().clone();
                self.queued_glyph_rects = GroupedRectsToPlace::new();
                true
            }
            Err(err) => match err {
                RectanglePackError::NotEnoughBinSpace => false,
            },
        }
    }

    fn render(&mut self) {
        for buffer in &mut self.bin_buffers {
            buffer.1.fill(0); // TODO: TEST CLEAR
        }

        for (glyph_index, (bin_id, location)) in &self.glyph_locations {
            let buffer = self.bin_buffers.get_mut(bin_id).unwrap();
            let bitmap = self
                .font_face
                .data()
                .glyph_bitmap(*glyph_index)
                .expect("Glyph not found in font but was placed!");
            let bin_pitch = self.atlases_size.x as usize;
            let offset = (location.y() * self.atlases_size.x + location.x()) as usize;
            let bitmap_data = match bitmap.data() {
                GlyphBitmapData::R8DistanceField(vec) => vec,
            };

            for x in 0..location.width() as usize {
                for y in 0..location.height() as usize {
                    buffer[offset + (y * bin_pitch) + x] =
                        bitmap_data[(y * location.width() as usize) + x];
                }
            }
        }

        for bin_id in self.bins.keys() {
            let texture = &self.bin_textures[bin_id];
            let buffer = &self.bin_buffers[bin_id];
            utils::copy_data_to_texture(
                &self.device,
                buffer,
                texture.desc.width,
                texture.desc.height,
                1,
                texture,
                ResourceState::Common,
            )
            .unwrap();
        }
    }

    fn add_bin(&mut self) {
        self.bins.insert(
            self.free_bin_id,
            TargetBin::new(self.atlases_size.x, self.atlases_size.y, 1),
        );
        self.bin_textures.insert(
            self.free_bin_id,
            self.device
                .create_texture(
                    &TextureDesc {
                        width: self.atlases_size.x,
                        height: self.atlases_size.y,
                        depth: 1,
                        mip_levels: 1,
                        format: PixelFormat::R8Unorm,
                        sample_desc: Default::default(),
                        usage_flags: TextureUsageFlags::empty(),
                        memory_location: MemoryLocation::GpuOnly,
                    },
                    &format!("Font Glyph Atlas {}", self.free_bin_id),
                )
                .unwrap(),
        );
        self.bin_buffers.insert(
            self.free_bin_id,
            vec![0u8; (self.atlases_size.x * self.atlases_size.y) as usize],
        );
        self.free_bin_id += 1;
    }

    fn _face(&self) -> &Arc<FontFace> {
        &self.font_face
    }
}
