use crate::maths::matrix::alignment::AlignStorage;
use crate::maths::matrix::MatrixNumber;
use crate::maths::Const;
use std::fmt::Debug;
use std::ops::{Index, IndexMut};

/// Trait implemented for every types that can store matrix data
pub trait Storage<T, R, C>: Debug + PartialEq + Clone {
    fn as_ptr(&self) -> *const T;
    fn as_mut_ptr(&mut self) -> *mut T;

    fn dimensions(&self) -> (R, C);

    /// # Safety
    ///
    /// Row and col must be valid indices
    unsafe fn get_unchecked(&self, row: usize, col: usize) -> &T {
        &*self.as_ptr().add(self.index(row, col))
    }

    /// # Safety
    ///
    /// Row and col must be valid indices
    unsafe fn get_unchecked_mut(&mut self, row: usize, col: usize) -> &mut T {
        &mut *self.as_mut_ptr().add(self.index(row, col))
    }

    /// Get index of the element at row/col
    fn index(&self, row: usize, col: usize) -> usize;
}

type AlignedRowType<T, const R: usize> = <T as AlignStorage>::RowType<R>;

#[repr(C)]
#[derive(Debug, Copy, Clone, PartialEq)]
/// Aligned column major matrix storage
pub struct AlignedColumnMajorStorage<T: AlignStorage, const R: usize, const C: usize>(
    pub [AlignedRowType<T, R>; C],
);

impl<T: MatrixNumber, const R: usize, const C: usize> Storage<T, Const<R>, Const<C>>
    for AlignedColumnMajorStorage<T, R, C>
{
    fn as_ptr(&self) -> *const T {
        self.0.as_ptr() as *const T
    }

    fn as_mut_ptr(&mut self) -> *mut T {
        self.0.as_mut_ptr() as *mut T
    }

    fn dimensions(&self) -> (Const<R>, Const<C>) {
        (Const::<R>, Const::<C>)
    }

    fn index(&self, row: usize, col: usize) -> usize {
        col * R + row
    }
}

impl<T: MatrixNumber, const R: usize, const C: usize> Default
    for AlignedColumnMajorStorage<T, R, C>
{
    fn default() -> Self {
        Self([Default::default(); C])
    }
}

impl<T: MatrixNumber, const R: usize, const C: usize> Index<usize>
    for AlignedColumnMajorStorage<T, R, C>
{
    type Output = AlignedRowType<T, R>;

    fn index(&self, index: usize) -> &Self::Output {
        &self.0[index]
    }
}

impl<T: MatrixNumber, const R: usize, const C: usize> IndexMut<usize>
    for AlignedColumnMajorStorage<T, R, C>
{
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.0[index]
    }
}

impl<T: MatrixNumber, const R: usize, const C: usize> From<[[T; R]; C]>
    for AlignedColumnMajorStorage<T, R, C>
{
    fn from(value: [[T; R]; C]) -> Self {
        Self(value.map(|row| row.into()))
    }
}

#[repr(transparent)]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
/// Non-aligned column major matrix storage
pub struct ColumnMajorStorage<T, const R: usize, const C: usize>(pub [[T; R]; C]);

impl<T: MatrixNumber, const R: usize, const C: usize> Storage<T, Const<R>, Const<C>>
    for ColumnMajorStorage<T, R, C>
{
    fn as_ptr(&self) -> *const T {
        self.0.as_ptr() as *const T
    }

    fn as_mut_ptr(&mut self) -> *mut T {
        self.0.as_mut_ptr() as *mut T
    }

    fn dimensions(&self) -> (Const<R>, Const<C>) {
        (Const::<R>, Const::<C>)
    }

    fn index(&self, row: usize, col: usize) -> usize {
        col * R + row
    }
}

impl<T: MatrixNumber, const R: usize, const C: usize> Default for ColumnMajorStorage<T, R, C> {
    fn default() -> Self {
        Self([[T::default(); R]; C])
    }
}

impl<T: MatrixNumber, const R: usize, const C: usize> Index<usize> for ColumnMajorStorage<T, R, C> {
    type Output = [T; R];

    fn index(&self, index: usize) -> &Self::Output {
        &self.0[index]
    }
}

impl<T: MatrixNumber, const R: usize, const C: usize> IndexMut<usize>
    for ColumnMajorStorage<T, R, C>
{
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.0[index]
    }
}

impl<T: MatrixNumber, const R: usize, const C: usize> From<[[T; R]; C]>
    for ColumnMajorStorage<T, R, C>
{
    fn from(value: [[T; R]; C]) -> Self {
        Self(value)
    }
}
