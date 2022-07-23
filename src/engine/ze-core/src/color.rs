#[derive(Copy, Clone, Default)]
pub struct Color4<T> {
    pub r: T,
    pub g: T,
    pub b: T,
    pub a: T,
}

impl<T> Color4<T> {
    pub fn new(r: T, g: T, b: T, a: T) -> Self {
        Self { r, g, b, a }
    }
}

pub type Color4f32 = Color4<f32>;

pub type Color4u8 = Color4<u8>;
impl From<Color4f32> for Color4u8 {
    fn from(color: Color4f32) -> Self {
        Self {
            r: (color.r * 255.0) as u8,
            g: (color.g * 255.0) as u8,
            b: (color.b * 255.0) as u8,
            a: (color.a * 255.0) as u8,
        }
    }
}
