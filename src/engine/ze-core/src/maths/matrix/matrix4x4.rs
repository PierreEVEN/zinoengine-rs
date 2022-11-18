use crate::maths::matrix::storage::Storage;
use crate::maths::matrix::{Matrix4x4, MatrixNumber};
use crate::maths::vector::Vector4;
use crate::maths::Inverse;
use std::arch::x86_64::*;
use std::ops::{Add, Div, Index, Mul, Neg, Sub};

impl<T: MatrixNumber> Matrix4x4<T> {
    #[inline]
    pub fn new(col0: Vector4<T>, col1: Vector4<T>, col2: Vector4<T>, col3: Vector4<T>) -> Self {
        Self {
            data: [col0.into(), col1.into(), col2.into(), col3.into()].into(),
            _phantom: Default::default(),
        }
    }
}

impl<T: MatrixNumber> Index<usize> for Matrix4x4<T> {
    type Output = Vector4<T>;

    fn index(&self, index: usize) -> &Self::Output {
        assert!(index < 4, "Index out of bounds");

        // SAFETY: We assert if the index is not valid
        unsafe { &*(self.data.get_unchecked(0, index) as *const T).cast::<Vector4<T>>() }
    }
}

impl<T> Inverse for Matrix4x4<T>
where
    Self: Mul<T, Output = Self>,
    T: MatrixNumber + Neg<Output = T>,
{
    fn try_inverse(self) -> Result<Self, ()> {
        // Inverse implementation from MESA
        let mut inverted = Self::default();
        inverted[(0, 0)] = self[(1, 1)] * self[(2, 2)] * self[(3, 3)]
            - self[(1, 1)] * self[(2, 3)] * self[(3, 2)]
            - self[(1, 2)] * self[(2, 1)] * self[(3, 3)]
            + self[(1, 2)] * self[(2, 3)] * self[(3, 1)]
            + self[(1, 3)] * self[(2, 1)] * self[(3, 2)]
            - self[(1, 3)] * self[(2, 2)] * self[(3, 1)];

        inverted[(0, 1)] = -self[(0, 1)] * self[(2, 2)] * self[(3, 3)]
            + self[(0, 1)] * self[(2, 3)] * self[(3, 2)]
            + self[(0, 2)] * self[(2, 1)] * self[(3, 3)]
            - self[(0, 2)] * self[(2, 3)] * self[(3, 1)]
            - self[(0, 3)] * self[(2, 1)] * self[(3, 2)]
            + self[(0, 3)] * self[(2, 2)] * self[(3, 1)];

        inverted[(0, 2)] = self[(0, 1)] * self[(1, 2)] * self[(3, 3)]
            - self[(0, 1)] * self[(1, 3)] * self[(3, 2)]
            - self[(0, 2)] * self[(1, 1)] * self[(3, 3)]
            + self[(0, 2)] * self[(1, 3)] * self[(3, 1)]
            + self[(0, 3)] * self[(1, 1)] * self[(3, 2)]
            - self[(0, 3)] * self[(1, 2)] * self[(3, 1)];

        inverted[(0, 3)] = -self[(0, 1)] * self[(1, 2)] * self[(2, 3)]
            + self[(0, 1)] * self[(1, 3)] * self[(2, 2)]
            + self[(0, 2)] * self[(1, 1)] * self[(2, 3)]
            - self[(0, 2)] * self[(1, 3)] * self[(2, 1)]
            - self[(0, 3)] * self[(1, 1)] * self[(2, 2)]
            + self[(0, 3)] * self[(1, 2)] * self[(2, 1)];

        inverted[(1, 0)] = -self[(1, 0)] * self[(2, 2)] * self[(3, 3)]
            + self[(1, 0)] * self[(2, 3)] * self[(3, 2)]
            + self[(1, 2)] * self[(2, 0)] * self[(3, 3)]
            - self[(1, 2)] * self[(2, 3)] * self[(3, 0)]
            - self[(1, 3)] * self[(2, 0)] * self[(3, 2)]
            + self[(1, 3)] * self[(2, 2)] * self[(3, 0)];

        inverted[(1, 1)] = self[(0, 0)] * self[(2, 2)] * self[(3, 3)]
            - self[(0, 0)] * self[(2, 3)] * self[(3, 2)]
            - self[(0, 2)] * self[(2, 0)] * self[(3, 3)]
            + self[(0, 2)] * self[(2, 3)] * self[(3, 0)]
            + self[(0, 3)] * self[(2, 0)] * self[(3, 2)]
            - self[(0, 3)] * self[(2, 2)] * self[(3, 0)];

        inverted[(1, 2)] = -self[(0, 0)] * self[(1, 2)] * self[(3, 3)]
            + self[(0, 0)] * self[(1, 3)] * self[(3, 2)]
            + self[(0, 2)] * self[(1, 0)] * self[(3, 3)]
            - self[(0, 2)] * self[(1, 3)] * self[(3, 0)]
            - self[(0, 3)] * self[(1, 0)] * self[(3, 2)]
            + self[(0, 3)] * self[(1, 2)] * self[(3, 0)];

        inverted[(1, 3)] = self[(0, 0)] * self[(1, 2)] * self[(2, 3)]
            - self[(0, 0)] * self[(1, 3)] * self[(2, 2)]
            - self[(0, 2)] * self[(1, 0)] * self[(2, 3)]
            + self[(0, 2)] * self[(1, 3)] * self[(2, 0)]
            + self[(0, 3)] * self[(1, 0)] * self[(2, 2)]
            - self[(0, 3)] * self[(1, 2)] * self[(2, 0)];

        inverted[(2, 0)] = self[(1, 0)] * self[(2, 1)] * self[(3, 3)]
            - self[(1, 0)] * self[(2, 3)] * self[(3, 1)]
            - self[(1, 1)] * self[(2, 0)] * self[(3, 3)]
            + self[(1, 1)] * self[(2, 3)] * self[(3, 0)]
            + self[(1, 3)] * self[(2, 0)] * self[(3, 1)]
            - self[(1, 3)] * self[(2, 1)] * self[(3, 0)];

        inverted[(2, 1)] = -self[(0, 0)] * self[(2, 1)] * self[(3, 3)]
            + self[(0, 0)] * self[(2, 3)] * self[(3, 1)]
            + self[(0, 1)] * self[(2, 0)] * self[(3, 3)]
            - self[(0, 1)] * self[(2, 3)] * self[(3, 0)]
            - self[(0, 3)] * self[(2, 0)] * self[(3, 1)]
            + self[(0, 3)] * self[(2, 1)] * self[(3, 0)];

        inverted[(2, 2)] = self[(0, 0)] * self[(1, 1)] * self[(3, 3)]
            - self[(0, 0)] * self[(1, 3)] * self[(3, 1)]
            - self[(0, 1)] * self[(1, 0)] * self[(3, 3)]
            + self[(0, 1)] * self[(1, 3)] * self[(3, 0)]
            + self[(0, 3)] * self[(1, 0)] * self[(3, 1)]
            - self[(0, 3)] * self[(1, 1)] * self[(3, 0)];

        inverted[(2, 3)] = -self[(0, 0)] * self[(1, 1)] * self[(2, 3)]
            + self[(0, 0)] * self[(1, 3)] * self[(2, 1)]
            + self[(0, 1)] * self[(1, 0)] * self[(2, 3)]
            - self[(0, 1)] * self[(1, 3)] * self[(2, 0)]
            - self[(0, 3)] * self[(1, 0)] * self[(2, 1)]
            + self[(0, 3)] * self[(1, 1)] * self[(2, 0)];

        inverted[(3, 0)] = -self[(1, 0)] * self[(2, 1)] * self[(3, 2)]
            + self[(1, 0)] * self[(2, 2)] * self[(3, 1)]
            + self[(1, 1)] * self[(2, 0)] * self[(3, 2)]
            - self[(1, 1)] * self[(2, 2)] * self[(3, 0)]
            - self[(1, 2)] * self[(2, 0)] * self[(3, 1)]
            + self[(1, 2)] * self[(2, 1)] * self[(3, 0)];

        inverted[(3, 1)] = self[(0, 0)] * self[(2, 1)] * self[(3, 2)]
            - self[(0, 0)] * self[(2, 2)] * self[(3, 1)]
            - self[(0, 1)] * self[(2, 0)] * self[(3, 2)]
            + self[(0, 1)] * self[(2, 2)] * self[(3, 0)]
            + self[(0, 2)] * self[(2, 0)] * self[(3, 1)]
            - self[(0, 2)] * self[(2, 1)] * self[(3, 0)];

        inverted[(3, 2)] = -self[(0, 0)] * self[(1, 1)] * self[(3, 2)]
            + self[(0, 0)] * self[(1, 2)] * self[(3, 1)]
            + self[(0, 1)] * self[(1, 0)] * self[(3, 2)]
            - self[(0, 1)] * self[(1, 2)] * self[(3, 0)]
            - self[(0, 2)] * self[(1, 0)] * self[(3, 1)]
            + self[(0, 2)] * self[(1, 1)] * self[(3, 0)];

        inverted[(3, 3)] = self[(0, 0)] * self[(1, 1)] * self[(2, 2)]
            - self[(0, 0)] * self[(1, 2)] * self[(2, 1)]
            - self[(0, 1)] * self[(1, 0)] * self[(2, 2)]
            + self[(0, 1)] * self[(1, 2)] * self[(2, 0)]
            + self[(0, 2)] * self[(1, 0)] * self[(2, 1)]
            - self[(0, 2)] * self[(1, 1)] * self[(2, 0)];

        let det = self[(0, 0)] * inverted[(0, 0)]
            + self[(0, 1)] * inverted[(1, 0)]
            + self[(0, 2)] * inverted[(2, 0)]
            + self[(0, 3)] * inverted[(3, 0)];

        if !det.is_zero() {
            let one_over_det = T::one() / det;
            let result = inverted * one_over_det;
            Ok(result)
        } else {
            Err(())
        }
    }
}

// SIMD arithmetic operators for f32
impl Add<Matrix4x4<f32>> for Matrix4x4<f32> {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        let mut result = Self::default();

        for i in 0..4 {
            unsafe {
                let result_row = result.data.get_unchecked_mut(0, i);
                _mm_store_ps(
                    result_row,
                    _mm_add_ps(
                        _mm_load_ps(self.data.get_unchecked(0, i)),
                        _mm_load_ps(rhs.data.get_unchecked(0, i)),
                    ),
                );
            }
        }

        result
    }
}

impl Sub<Matrix4x4<f32>> for Matrix4x4<f32> {
    type Output = Self;

    fn sub(self, rhs: Matrix4x4<f32>) -> Self::Output {
        let mut result = Self::default();

        for i in 0..4 {
            unsafe {
                let result_row = result.data.get_unchecked_mut(0, i);
                _mm_store_ps(
                    result_row,
                    _mm_sub_ps(
                        _mm_load_ps(self.data.get_unchecked(0, i)),
                        _mm_load_ps(rhs.data.get_unchecked(0, i)),
                    ),
                );
            }
        }

        result
    }
}

impl Mul<Matrix4x4<f32>> for Matrix4x4<f32> {
    type Output = Self;

    #[inline]
    fn mul(self, rhs: Matrix4x4<f32>) -> Self::Output {
        let mut result = Self::default();

        let row1 = unsafe { _mm_load_ps(rhs.data.get_unchecked(0, 0)) };
        let row2 = unsafe { _mm_load_ps(rhs.data.get_unchecked(0, 1)) };
        let row3 = unsafe { _mm_load_ps(rhs.data.get_unchecked(0, 2)) };
        let row4 = unsafe { _mm_load_ps(rhs.data.get_unchecked(0, 3)) };

        for i in 0..4 {
            unsafe {
                let left_row0 = _mm_set1_ps(*self.data.get_unchecked(0, i));
                let left_row1 = _mm_set1_ps(*self.data.get_unchecked(1, i));
                let left_row2 = _mm_set1_ps(*self.data.get_unchecked(2, i));
                let left_row3 = _mm_set1_ps(*self.data.get_unchecked(3, i));
                let result_row = result.data.get_unchecked_mut(0, i);

                _mm_store_ps(
                    result_row,
                    _mm_add_ps(
                        _mm_add_ps(_mm_mul_ps(left_row0, row1), _mm_mul_ps(left_row1, row2)),
                        _mm_add_ps(_mm_mul_ps(left_row2, row3), _mm_mul_ps(left_row3, row4)),
                    ),
                );
            }
        }

        result
    }
}

impl Mul<f32> for Matrix4x4<f32> {
    type Output = Self;

    #[inline]
    fn mul(self, rhs: f32) -> Self::Output {
        let mut result = Self::default();
        let rhs_row = unsafe { _mm_set1_ps(rhs) };

        for i in 0..4 {
            unsafe {
                let row = _mm_load_ps(self.data.get_unchecked(0, i));
                _mm_store_ps(
                    result.data.get_unchecked_mut(0, i),
                    _mm_mul_ps(row, rhs_row),
                );
            }
        }

        result
    }
}

impl Div<Matrix4x4<f32>> for Matrix4x4<f32> {
    type Output = Self;

    #[allow(clippy::suspicious_arithmetic_impl)]
    fn div(self, rhs: Matrix4x4<f32>) -> Self::Output {
        self * rhs.inverse()
    }
}

// SIMD arithmetic operators for f64
impl Add<Matrix4x4<f64>> for Matrix4x4<f64> {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        let mut result = Self::default();

        for i in 0..4 {
            unsafe {
                let result_row = result.data.get_unchecked_mut(0, i);
                _mm256_store_pd(
                    result_row,
                    _mm256_add_pd(
                        _mm256_load_pd(self.data.get_unchecked(0, i)),
                        _mm256_load_pd(rhs.data.get_unchecked(0, i)),
                    ),
                );
            }
        }

        result
    }
}

impl Sub<Matrix4x4<f64>> for Matrix4x4<f64> {
    type Output = Self;

    fn sub(self, rhs: Matrix4x4<f64>) -> Self::Output {
        let mut result = Self::default();

        for i in 0..4 {
            unsafe {
                let result_row = result.data.get_unchecked_mut(0, i);
                _mm256_store_pd(
                    result_row,
                    _mm256_sub_pd(
                        _mm256_load_pd(self.data.get_unchecked(0, i)),
                        _mm256_load_pd(rhs.data.get_unchecked(0, i)),
                    ),
                );
            }
        }

        result
    }
}

impl Mul<Matrix4x4<f64>> for Matrix4x4<f64> {
    type Output = Self;

    #[inline]
    fn mul(self, rhs: Matrix4x4<f64>) -> Self::Output {
        let mut result = Self::default();

        let row1 = unsafe { _mm256_load_pd(rhs.data.get_unchecked(0, 0)) };
        let row2 = unsafe { _mm256_load_pd(rhs.data.get_unchecked(0, 1)) };
        let row3 = unsafe { _mm256_load_pd(rhs.data.get_unchecked(0, 2)) };
        let row4 = unsafe { _mm256_load_pd(rhs.data.get_unchecked(0, 3)) };

        for i in 0..4 {
            unsafe {
                let left_row0 = _mm256_set1_pd(*self.data.get_unchecked(0, i));
                let left_row1 = _mm256_set1_pd(*self.data.get_unchecked(1, i));
                let left_row2 = _mm256_set1_pd(*self.data.get_unchecked(2, i));
                let left_row3 = _mm256_set1_pd(*self.data.get_unchecked(3, i));
                let result_row = result.data.get_unchecked_mut(0, i);

                _mm256_store_pd(
                    result_row,
                    _mm256_add_pd(
                        _mm256_add_pd(
                            _mm256_mul_pd(left_row0, row1),
                            _mm256_mul_pd(left_row1, row2),
                        ),
                        _mm256_add_pd(
                            _mm256_mul_pd(left_row2, row3),
                            _mm256_mul_pd(left_row3, row4),
                        ),
                    ),
                );
            }
        }

        result
    }
}

impl Mul<f64> for Matrix4x4<f64> {
    type Output = Self;

    #[inline]
    fn mul(self, rhs: f64) -> Self::Output {
        let mut result = Self::default();
        let rhs_row = unsafe { _mm256_set1_pd(rhs) };

        for i in 0..4 {
            unsafe {
                let row = _mm256_load_pd(self.data.get_unchecked(0, i));
                _mm256_store_pd(
                    result.data.get_unchecked_mut(0, i),
                    _mm256_mul_pd(row, rhs_row),
                );
            }
        }

        result
    }
}

impl Div<Matrix4x4<f64>> for Matrix4x4<f64> {
    type Output = Self;

    #[allow(clippy::suspicious_arithmetic_impl)]
    fn div(self, rhs: Matrix4x4<f64>) -> Self::Output {
        self * rhs.inverse()
    }
}

#[cfg(test)]
mod tests {
    mod f32 {
        use crate::maths::{Inverse, Matrix4x4};

        #[test]
        fn add() {
            let a = Matrix4x4::<f32>::from([
                [1.0, 2.0, 3.0, 4.0],
                [1.0, 2.0, 3.0, 4.0],
                [1.0, 2.0, 3.0, 4.0],
                [1.0, 2.0, 3.0, 4.0],
            ]);
            let b = Matrix4x4::<f32>::from([
                [1.0, 2.0, 3.0, 4.0],
                [1.0, 2.0, 3.0, 4.0],
                [1.0, 2.0, 3.0, 4.0],
                [1.0, 2.0, 3.0, 4.0],
            ]);
            assert_eq!(
                a + b,
                Matrix4x4::<f32>::from([
                    [2.0, 4.0, 6.0, 8.0],
                    [2.0, 4.0, 6.0, 8.0],
                    [2.0, 4.0, 6.0, 8.0],
                    [2.0, 4.0, 6.0, 8.0],
                ])
            );
        }

        #[test]
        fn sub() {
            let a = Matrix4x4::<f32>::from([
                [1.0, 2.0, 3.0, 4.0],
                [1.0, 2.0, 3.0, 4.0],
                [1.0, 2.0, 3.0, 4.0],
                [1.0, 2.0, 3.0, 4.0],
            ]);
            let b = Matrix4x4::<f32>::from([
                [1.0, 2.0, 3.0, 4.0],
                [1.0, 2.0, 3.0, 4.0],
                [1.0, 2.0, 3.0, 4.0],
                [1.0, 2.0, 3.0, 3.0],
            ]);
            assert_eq!(
                a - b,
                Matrix4x4::<f32>::from([
                    [0.0, 0.0, 0.0, 0.0],
                    [0.0, 0.0, 0.0, 0.0],
                    [0.0, 0.0, 0.0, 0.0],
                    [0.0, 0.0, 0.0, 1.0],
                ])
            );
        }

        #[test]
        fn mul() {
            let a = Matrix4x4::<f32>::from([
                [1.0, 2.0, 3.0, 3.0],
                [1.0, 2.0, 3.0, 3.0],
                [1.0, 2.0, 3.0, 3.0],
                [1.0, 2.0, 3.0, 3.0],
            ]);
            let b = Matrix4x4::<f32>::from([
                [1.0, 2.0, 3.0, 3.0],
                [1.0, 2.0, 3.0, 3.0],
                [1.0, 2.0, 3.0, 3.0],
                [1.0, 2.0, 3.0, 3.0],
            ]);
            let c = a * b;
            assert_eq!(
                c,
                Matrix4x4::<f32>::from([
                    [9.0, 18.0, 27.0, 27.0],
                    [9.0, 18.0, 27.0, 27.0],
                    [9.0, 18.0, 27.0, 27.0],
                    [9.0, 18.0, 27.0, 27.0]
                ])
            );
        }

        #[test]
        fn mul_with_scalar() {
            let a = Matrix4x4::<f32>::from([
                [1.0, 2.0, 3.0, 3.0],
                [1.0, 2.0, 3.0, 3.0],
                [1.0, 2.0, 3.0, 3.0],
                [1.0, 2.0, 3.0, 3.0],
            ]);
            let c = a * 2.0;
            assert_eq!(
                c,
                Matrix4x4::<f32>::from([
                    [2.0, 4.0, 6.0, 6.0],
                    [2.0, 4.0, 6.0, 6.0],
                    [2.0, 4.0, 6.0, 6.0],
                    [2.0, 4.0, 6.0, 6.0]
                ])
            );
        }

        #[test]
        fn div() {
            let a = Matrix4x4::<f32>::from([
                [6.0, 4.0, 3.0, 4.0],
                [1.0, 2.0, 4.0, 4.0],
                [1.0, 2.0, 2.0, 2.0],
                [4.0, 4.0, 3.0, 3.0],
            ]);
            let b = Matrix4x4::<f32>::from([
                [6.0, 4.0, 3.0, 4.0],
                [1.0, 2.0, 4.0, 4.0],
                [1.0, 2.0, 2.0, 2.0],
                [4.0, 4.0, 3.0, 3.0],
            ]);
            let c = a / b;
            assert_eq!(c, Matrix4x4::<f32>::identity());
        }

        #[test]
        #[should_panic]
        fn inverse_not_invertible_panic() {
            let a = Matrix4x4::<f32>::from([
                [1.0, 2.0, 3.0, 3.0],
                [1.0, 2.0, 3.0, 3.0],
                [1.0, 2.0, 3.0, 3.0],
                [1.0, 2.0, 3.0, 3.0],
            ]);
            a.try_inverse().unwrap();
        }

        #[test]
        fn inverse() {
            let a = Matrix4x4::<f32>::from([
                [6.0, 4.0, 3.0, 4.0],
                [1.0, 2.0, 4.0, 4.0],
                [1.0, 2.0, 2.0, 2.0],
                [4.0, 4.0, 3.0, 3.0],
            ]);
            let a = a.inverse();
            assert_eq!(
                a,
                Matrix4x4::<f32>::from([
                    [0.0, 1.0 / 4.0, -5.0 / 4.0, 1.0 / 2.0],
                    [0.0, -5.0 / 8.0, 13.0 / 8.0, -1.0 / 4.0],
                    [-1.0, 1.0, -3.0, 2.0],
                    [1.0, -1.0 / 2.0, 5.0 / 2.0, -2.0],
                ])
            );
        }
    }

    mod f64 {
        use crate::maths::{Inverse, Matrix4x4};

        #[test]
        fn add() {
            let a = Matrix4x4::<f64>::from([
                [1.0, 2.0, 3.0, 4.0],
                [1.0, 2.0, 3.0, 4.0],
                [1.0, 2.0, 3.0, 4.0],
                [1.0, 2.0, 3.0, 4.0],
            ]);
            let b = Matrix4x4::<f64>::from([
                [1.0, 2.0, 3.0, 4.0],
                [1.0, 2.0, 3.0, 4.0],
                [1.0, 2.0, 3.0, 4.0],
                [1.0, 2.0, 3.0, 4.0],
            ]);
            assert_eq!(
                a + b,
                Matrix4x4::<f64>::from([
                    [2.0, 4.0, 6.0, 8.0],
                    [2.0, 4.0, 6.0, 8.0],
                    [2.0, 4.0, 6.0, 8.0],
                    [2.0, 4.0, 6.0, 8.0],
                ])
            );
        }

        #[test]
        fn sub() {
            let a = Matrix4x4::<f64>::from([
                [1.0, 2.0, 3.0, 4.0],
                [1.0, 2.0, 3.0, 4.0],
                [1.0, 2.0, 3.0, 4.0],
                [1.0, 2.0, 3.0, 4.0],
            ]);
            let b = Matrix4x4::<f64>::from([
                [1.0, 2.0, 3.0, 4.0],
                [1.0, 2.0, 3.0, 4.0],
                [1.0, 2.0, 3.0, 4.0],
                [1.0, 2.0, 3.0, 3.0],
            ]);
            assert_eq!(
                a - b,
                Matrix4x4::<f64>::from([
                    [0.0, 0.0, 0.0, 0.0],
                    [0.0, 0.0, 0.0, 0.0],
                    [0.0, 0.0, 0.0, 0.0],
                    [0.0, 0.0, 0.0, 1.0],
                ])
            );
        }

        #[test]
        fn mul() {
            let a = Matrix4x4::<f64>::from([
                [1.0, 2.0, 3.0, 3.0],
                [1.0, 2.0, 3.0, 3.0],
                [1.0, 2.0, 3.0, 3.0],
                [1.0, 2.0, 3.0, 3.0],
            ]);
            let b = Matrix4x4::<f64>::from([
                [1.0, 2.0, 3.0, 3.0],
                [1.0, 2.0, 3.0, 3.0],
                [1.0, 2.0, 3.0, 3.0],
                [1.0, 2.0, 3.0, 3.0],
            ]);
            let c = a * b;
            assert_eq!(
                c,
                Matrix4x4::<f64>::from([
                    [9.0, 18.0, 27.0, 27.0],
                    [9.0, 18.0, 27.0, 27.0],
                    [9.0, 18.0, 27.0, 27.0],
                    [9.0, 18.0, 27.0, 27.0]
                ])
            );
        }

        #[test]
        fn mul_with_scalar() {
            let a = Matrix4x4::<f64>::from([
                [1.0, 2.0, 3.0, 3.0],
                [1.0, 2.0, 3.0, 3.0],
                [1.0, 2.0, 3.0, 3.0],
                [1.0, 2.0, 3.0, 3.0],
            ]);
            let c = a * 2.0;
            assert_eq!(
                c,
                Matrix4x4::<f64>::from([
                    [2.0, 4.0, 6.0, 6.0],
                    [2.0, 4.0, 6.0, 6.0],
                    [2.0, 4.0, 6.0, 6.0],
                    [2.0, 4.0, 6.0, 6.0]
                ])
            );
        }

        #[test]
        fn div() {
            let a = Matrix4x4::<f64>::from([
                [6.0, 4.0, 3.0, 4.0],
                [1.0, 2.0, 4.0, 4.0],
                [1.0, 2.0, 2.0, 2.0],
                [4.0, 4.0, 3.0, 3.0],
            ]);
            let b = Matrix4x4::<f64>::from([
                [6.0, 4.0, 3.0, 4.0],
                [1.0, 2.0, 4.0, 4.0],
                [1.0, 2.0, 2.0, 2.0],
                [4.0, 4.0, 3.0, 3.0],
            ]);
            let c = a / b;
            assert_eq!(c, Matrix4x4::<f64>::identity());
        }

        #[test]
        #[should_panic]
        fn inverse_not_invertible_panic() {
            let a = Matrix4x4::<f64>::from([
                [1.0, 2.0, 3.0, 3.0],
                [1.0, 2.0, 3.0, 3.0],
                [1.0, 2.0, 3.0, 3.0],
                [1.0, 2.0, 3.0, 3.0],
            ]);
            a.try_inverse().unwrap();
        }

        #[test]
        fn inverse() {
            let a = Matrix4x4::<f64>::from([
                [6.0, 4.0, 3.0, 4.0],
                [1.0, 2.0, 4.0, 4.0],
                [1.0, 2.0, 2.0, 2.0],
                [4.0, 4.0, 3.0, 3.0],
            ]);
            let a = a.inverse();
            assert_eq!(
                a,
                Matrix4x4::<f64>::from([
                    [0.0, 1.0 / 4.0, -5.0 / 4.0, 1.0 / 2.0],
                    [0.0, -5.0 / 8.0, 13.0 / 8.0, -1.0 / 4.0],
                    [-1.0, 1.0, -3.0, 2.0],
                    [1.0, -1.0 / 2.0, 5.0 / 2.0, -2.0],
                ])
            );
        }
    }
}
