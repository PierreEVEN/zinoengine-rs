pub enum TextureFormat {
    R8G8B8A8,
    R8G8B8A8sRGB,
}

/// Raw texture format containing all data to compile an asset
pub struct RawTexture {
    format: TextureFormat,
    pixels: Vec<u8>,
}

pub struct Texture {}
