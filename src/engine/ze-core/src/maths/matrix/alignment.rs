use std::fmt::Debug;

/// Trait implemented by types so they can tell what appropriate row type to use for a correct alignment
pub trait AlignStorage: Sized {
    type RowType<const N: usize>: Copy + Clone + Debug + PartialEq + Default + From<[Self; N]>;
}

#[repr(C, align(16))]
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct AlignedArray16<T, const R: usize>(pub [T; R]);

impl<T: Default + Copy, const R: usize> Default for AlignedArray16<T, R> {
    fn default() -> Self {
        Self([Default::default(); R])
    }
}

impl<T, const R: usize> From<[T; R]> for AlignedArray16<T, R> {
    fn from(array: [T; R]) -> Self {
        Self(array)
    }
}

#[repr(C, align(32))]
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct AlignedArray32<T, const R: usize>(pub [T; R]);

impl<T: Default + Copy, const R: usize> Default for AlignedArray32<T, R> {
    fn default() -> Self {
        Self([Default::default(); R])
    }
}

impl<T, const R: usize> From<[T; R]> for AlignedArray32<T, R> {
    fn from(array: [T; R]) -> Self {
        Self(array)
    }
}

// Align storage implementations

impl AlignStorage for u32 {
    type RowType<const R: usize> = AlignedArray16<Self, R>;
}

impl AlignStorage for i32 {
    type RowType<const R: usize> = AlignedArray16<Self, R>;
}

impl AlignStorage for f32 {
    type RowType<const R: usize> = AlignedArray16<Self, R>;
}

impl AlignStorage for f64 {
    type RowType<const R: usize> = AlignedArray32<Self, R>;
}
