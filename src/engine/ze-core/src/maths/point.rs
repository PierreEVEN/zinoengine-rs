use crate::impl_component_deref;
use crate::maths::vector::*;
use crate::maths::{MatrixNumber, Vector};
use std::ops::Deref;
use std::ops::DerefMut;

/// A point in euclidean space
#[repr(transparent)]
#[derive(Copy, Clone, PartialEq, Debug, Default)]
pub struct Point<T: MatrixNumber, const D: usize> {
    pub coordinates: Vector<T, D>,
}

pub type Point2<T> = Point<T, 2>;

impl<T: MatrixNumber> Point<T, 2> {
    pub fn new(x: T, y: T) -> Self {
        Self {
            coordinates: Vector::<T, 2>::new(x, y),
        }
    }
}

impl_component_deref!(Point 1 -> X);
impl_component_deref!(Point 2 -> XY);
impl_component_deref!(Point 3 -> XYZ);
impl_component_deref!(Point 4 -> XYZW);
