use num_derive::FromPrimitive;
use num_traits::FromPrimitive;
use serde_derive::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};
use ze_reflection::*;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize, FromPrimitive, Reflectable)]
#[non_exhaustive]
pub enum PixelFormat {
    Unknown,
    B8G8R8A8UnormSrgb,
    B8G8R8A8Unorm,

    R8Unorm,

    R8G8B8Unorm,
    R8G8B8A8Unorm,
}

impl PixelFormat {
    pub fn bytes_size(&self) -> u64 {
        match self {
            PixelFormat::Unknown => 0,
            PixelFormat::B8G8R8A8UnormSrgb
            | PixelFormat::B8G8R8A8Unorm
            | PixelFormat::R8G8B8A8Unorm => 4,

            PixelFormat::R8Unorm => 1,

            PixelFormat::R8G8B8Unorm => 3,
        }
    }

    pub fn texture_size_in_bytes(&self, width: u32, height: u32) -> u64 {
        (width as u64) * (height as u64) * self.bytes_size()
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
            PixelFormat::R8G8B8Unorm => write!(f, "RGB 8-bit (unorm)"),
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
}

pub mod backend;
pub mod utils;
