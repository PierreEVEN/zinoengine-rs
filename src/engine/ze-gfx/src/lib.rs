use num_derive::FromPrimitive;
use num_traits::FromPrimitive;
use serde_derive::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};
use ze_reflection::*;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize, FromPrimitive, Reflectable)]
#[non_exhaustive]
pub enum PixelFormat {
    Unknown,

    // BGRA formats
    B8G8R8A8UnormSrgb,
    B8G8R8A8Unorm,

    // R formats
    R8Unorm,

    // RGBA formats
    R8G8B8A8Unorm,

    // Depth/stencil formats
    D24UnormS8Uint,
}

impl PixelFormat {
    pub fn bytes_size(&self) -> usize {
        match self {
            PixelFormat::Unknown => 0,
            PixelFormat::B8G8R8A8UnormSrgb
            | PixelFormat::B8G8R8A8Unorm
            | PixelFormat::R8G8B8A8Unorm => 4,

            PixelFormat::R8Unorm => 1,

            // Depth/stencil formats
            PixelFormat::D24UnormS8Uint => 4,
        }
    }

    pub fn texture_size_in_bytes(&self, width: u32, height: u32) -> usize {
        (width as usize) * (height as usize) * self.bytes_size()
    }
}

impl Default for PixelFormat {
    fn default() -> Self {
        Self::Unknown
    }
}

impl Display for PixelFormat {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            PixelFormat::B8G8R8A8UnormSrgb => write!(f, "BGRA 8-bit (unorm, sRGB)"),
            PixelFormat::B8G8R8A8Unorm => write!(f, "BGRA 8-bit (unorm)"),
            PixelFormat::R8G8B8A8Unorm => write!(f, "RGBA 8-bit (unorm)"),
            _ => write!(f, "{:?}", self),
        }
    }
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
    Mesh = 1 << 3,
}

pub mod backend;
pub mod null;
pub mod utils;
