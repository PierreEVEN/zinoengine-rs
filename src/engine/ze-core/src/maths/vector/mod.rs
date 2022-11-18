mod component;

use crate::impl_component_deref;
use crate::maths::storage::ColumnMajorStorage;
use crate::maths::{Const, Matrix, MatrixNumber};
use std::ops::{Add, Deref, DerefMut, Div, Index, IndexMut, Mul, Sub};

pub use component::*;

/// A mathematical vector, internally represented as a 1-column matrix
#[repr(transparent)]
#[derive(Debug, Default, Copy, Clone, PartialEq)]
pub struct Vector<T: MatrixNumber, const D: usize>(
    Matrix<T, Const<D>, Const<1>, ColumnMajorStorage<T, D, 1>>,
);

impl<T: MatrixNumber, const D: usize> Index<usize> for Vector<T, D> {
    type Output = T;

    fn index(&self, index: usize) -> &Self::Output {
        &self.0[(index, 0)]
    }
}

impl<T: MatrixNumber, const D: usize> IndexMut<usize> for Vector<T, D> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.0[(index, 0)]
    }
}

impl<T: MatrixNumber, const D: usize> Add<Self> for Vector<T, D> {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        let mut result = Self::default();
        for i in 0..D {
            result[i] = self[i] + rhs[i];
        }
        result
    }
}

impl<T: MatrixNumber, const D: usize> Sub<Self> for Vector<T, D> {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        let mut result = Self::default();
        for i in 0..D {
            result[i] = self[i] - rhs[i];
        }
        result
    }
}

impl<T: MatrixNumber, const D: usize> Mul<Self> for Vector<T, D> {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self::Output {
        let mut result = Self::default();
        for i in 0..D {
            result[i] = self[i] * rhs[i];
        }
        result
    }
}

impl<T: MatrixNumber, const D: usize> Div<Self> for Vector<T, D> {
    type Output = Self;

    fn div(self, rhs: Self) -> Self::Output {
        let mut result = Self::default();
        for i in 0..D {
            result[i] = self[i] / rhs[i];
        }
        result
    }
}

impl<T: MatrixNumber, const D: usize>
    AsRef<Matrix<T, Const<D>, Const<1>, ColumnMajorStorage<T, D, 1>>> for Vector<T, D>
{
    fn as_ref(&self) -> &Matrix<T, Const<D>, Const<1>, ColumnMajorStorage<T, D, 1>> {
        &self.0
    }
}

// Vector aliases

pub type Vector2<T> = Vector<T, 2>;

impl<T: MatrixNumber> Vector2<T> {
    pub fn new(x: T, y: T) -> Self {
        Self([[x, y]].into())
    }
}

impl<T: MatrixNumber> From<Vector2<T>> for [T; 2] {
    fn from(value: Vector2<T>) -> Self {
        [value.x, value.y]
    }
}

pub type Vector3<T> = Vector<T, 3>;

impl<T: MatrixNumber> Vector3<T> {
    pub fn new(x: T, y: T, z: T) -> Self {
        Self([[x, y, z]].into())
    }
}

impl<T: MatrixNumber> From<Vector3<T>> for [T; 3] {
    fn from(value: Vector3<T>) -> Self {
        [value.x, value.y, value.z]
    }
}

pub type Vector4<T> = Vector<T, 4>;

impl<T: MatrixNumber> Vector4<T> {
    pub fn new(x: T, y: T, z: T, w: T) -> Self {
        Self([[x, y, z, w]].into())
    }
}

impl<T: MatrixNumber> From<Vector4<T>> for [T; 4] {
    fn from(value: Vector4<T>) -> Self {
        [value.x, value.y, value.z, value.w]
    }
}

impl_component_deref!(Vector 1 -> X);
impl_component_deref!(Vector 2 -> XY);
impl_component_deref!(Vector 3 -> XYZ);
impl_component_deref!(Vector 4 -> XYZW);

#[cfg(test)]
mod tests {
    use crate::maths::vector::Vector4;

    #[test]
    fn add() {
        let a = Vector4::new(1, 2, 3, 4);
        let b = Vector4::new(5, 6, 7, 8);
        let c = a + b;
        assert_eq!(c, Vector4::new(6, 8, 10, 12));
    }

    #[test]
    fn sub() {
        let a = Vector4::new(1, 2, 3, 4);
        let b = Vector4::new(5, 6, 7, 8);
        let c = a - b;
        assert_eq!(c, Vector4::new(-4, -4, -4, -4));
    }

    #[test]
    fn mul() {
        let a = Vector4::new(1, 2, 3, 4);
        let b = Vector4::new(5, 6, 7, 8);
        let c = a * b;
        assert_eq!(c, Vector4::new(5, 12, 21, 32));
    }

    #[test]
    fn div() {
        let a = Vector4::new(1, 2, 3, 4);
        let b = Vector4::new(5, 6, 7, 8);
        let c = a / b;
        assert_eq!(c, Vector4::new(1 / 5, 2 / 6, 3 / 7, 4 / 8));
    }

    #[test]
    fn component_access() {
        let mut v = Vector4::new(1, 2, 3, 4);
        assert_eq!(v.x, 1);
        assert_eq!(v.y, 2);
        assert_eq!(v.z, 3);
        assert_eq!(v.w, 4);

        v.x = 5;
        v.y = 6;
        v.z = 7;
        v.w = 8;

        assert_eq!(v.x, 5);
        assert_eq!(v.y, 6);
        assert_eq!(v.z, 7);
        assert_eq!(v.w, 8);
    }
}
