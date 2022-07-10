pub mod backend;
pub mod utils;

#[derive(Copy, Clone, Debug)]
#[non_exhaustive]
pub enum PixelFormat {
    Unknown,
    B8G8R8A8UnormSrgb,
    B8G8R8A8Unorm,
    R8G8B8A8Unorm,
}

#[derive(Copy, Clone, Debug)]
pub struct SampleDesc {
    pub count: u32,
    pub quality: u32,
}

impl Default for SampleDesc {
    fn default() -> Self {
        Self {
            count: 1,
            quality: 0,
        }
    }
}

#[derive(PartialEq, Eq, Copy, Clone, Debug)]
pub enum ShaderStageFlagBits {
    Vertex = 1 << 0,
    Fragment = 1 << 1,
    Compute = 1 << 2,
}
