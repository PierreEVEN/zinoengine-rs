mod matrix;
mod point;
mod vector;

#[derive(Ord, PartialOrd, Eq, PartialEq, Debug, Default, Copy, Clone)]
pub struct Rect<T: Default> {
    pub x: T,
    pub y: T,
    pub width: T,
    pub height: T,
}

impl<T: Default> Rect<T> {
    pub fn new(x: T, y: T, width: T, height: T) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }
}

/// Represent a dimension
pub trait Dim: Copy + Clone {
    const VALUE: usize;

    fn value(&self) -> usize;
}

pub trait Inverse: Sized {
    #[allow(clippy::result_unit_err)]
    fn try_inverse(self) -> Result<Self, ()>;

    /// Try inverse the object
    ///
    /// # Panics
    ///
    /// Panic if it is not invertible
    fn inverse(self) -> Self {
        self.try_inverse().expect("Not invertible")
    }
}

/// Represent a constant value
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Const<const T: usize>;

impl<const T: usize> Dim for Const<T> {
    const VALUE: usize = T;

    fn value(&self) -> usize {
        T
    }
}

pub type RectF32 = Rect<f32>;
pub type RectI32 = Rect<i32>;

pub use matrix::*;
pub use point::*;
pub use vector::*;
