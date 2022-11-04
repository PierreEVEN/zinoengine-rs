mod alignment;
pub mod matrix4x4;
pub mod storage;

use crate::maths::matrix::alignment::AlignStorage;
use crate::maths::matrix::storage::{AlignedColumnMajorStorage, Storage};
use crate::maths::{Const, Dim};
use num_traits::Num;
use std::fmt::Debug;
use std::marker::PhantomData;
use std::ops::{Index, IndexMut};

/// Number value usable for a matrix
pub trait MatrixNumber: Num + Debug + Copy + Default + AlignStorage {}
impl MatrixNumber for u32 {}
impl MatrixNumber for i32 {}
impl MatrixNumber for f32 {}
impl MatrixNumber for f64 {}

/// A matrix with a fixed number of rows and columns
#[repr(transparent)]
#[derive(Debug, Copy, Clone, Default, PartialEq)]
pub struct Matrix<T: MatrixNumber, R: Dim, C: Dim, S: Storage<T, R, C>> {
    pub data: S,
    _phantom: PhantomData<(T, R, C, S)>,
}

impl<T, R, C, S> Matrix<T, R, C, S>
where
    T: MatrixNumber,
    R: Dim,
    C: Dim,
    S: Storage<T, R, C>,
{
    /// Returns the identity matrix
    ///
    /// # Example
    ///
    /// ```
    /// use ze_core::maths::Matrix4x4;
    ///
    /// let m = Matrix4x4::<f32>::identity();
    /// assert_eq!(m, Matrix4x4::from([
    ///     [1.0, 0.0, 0.0, 0.0],
    ///     [0.0, 1.0, 0.0, 0.0],
    ///     [0.0, 0.0, 1.0, 0.0],
    ///     [0.0, 0.0, 0.0, 1.0]
    /// ]));
    /// ```
    #[inline]
    pub fn identity() -> Self
    where
        Self: Default,
    {
        let mut result = Self::default();
        for i in 0..R::VALUE {
            result[(i, i)] = T::one();
        }
        result
    }

    /// Transpose the matrix, effectively inverting the rows and columns
    ///
    /// # Example
    ///
    /// ```
    /// use ze_core::maths::Matrix4x4;
    ///
    /// let mut m = Matrix4x4::<f32>::from([
    ///     [1.0, 2.0, 3.0, 4.0],
    ///     [5.0, 6.0, 7.0, 8.0],
    ///     [9.0, 10.0, 11.0, 12.0],
    ///     [13.0, 14.0, 15.0, 16.0],
    /// ]);
    /// m.transpose();
    /// assert_eq!(m, Matrix4x4::from([
    ///     [1.0, 5.0, 9.0, 13.0],
    ///     [2.0, 6.0, 10.0, 14.0],
    ///     [3.0, 7.0, 11.0, 15.0],
    ///     [4.0, 8.0, 12.0, 16.0]
    /// ]));
    /// ```
    pub fn transpose(&mut self) {
        let other = self.clone();

        for r in 0..R::VALUE {
            for c in 0..C::VALUE {
                self[(r, c)] = other[(c, r)];
            }
        }
    }
}

impl<T, R, C, S> Index<(usize, usize)> for Matrix<T, R, C, S>
where
    T: MatrixNumber,
    R: Dim,
    C: Dim,
    S: Storage<T, R, C>,
{
    type Output = T;

    #[inline]
    fn index(&self, index: (usize, usize)) -> &Self::Output {
        assert!(
            index.0 < R::VALUE && index.1 < C::VALUE,
            "Index out of bounds"
        );

        // SAFETY: We assert if the index is not valid
        unsafe { self.data.get_unchecked(index.0, index.1) }
    }
}

impl<T, R, C, S> IndexMut<(usize, usize)> for Matrix<T, R, C, S>
where
    T: MatrixNumber,
    R: Dim,
    C: Dim,
    S: Storage<T, R, C>,
{
    #[inline]
    fn index_mut(&mut self, index: (usize, usize)) -> &mut Self::Output {
        assert!(
            index.0 < R::VALUE && index.1 < C::VALUE,
            "Index out of bounds"
        );

        // SAFETY: We assert if the index is not valid
        unsafe { self.data.get_unchecked_mut(index.0, index.1) }
    }
}

impl<T, const R: usize, const C: usize, S> From<[[T; R]; C]> for Matrix<T, Const<R>, Const<C>, S>
where
    T: MatrixNumber,
    S: Storage<T, Const<R>, Const<C>> + From<[[T; R]; C]>,
{
    #[inline]
    fn from(data: [[T; R]; C]) -> Self {
        Self {
            data: data.into(),
            _phantom: Default::default(),
        }
    }
}

/// Column-major 4x4 matrix
pub type Matrix4x4<T> = Matrix<T, Const<4>, Const<4>, AlignedColumnMajorStorage<T, 4, 4>>;
