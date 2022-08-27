use num_traits::{Num, NumAssign};
use std::ops::{Add, AddAssign, Div, DivAssign, Index, IndexMut, Mul, MulAssign, Sub, SubAssign};

/// A two-dimensional vector that can either represent a direction or a point in space
#[derive(Ord, PartialOrd, Eq, PartialEq, Debug, Default, Copy, Clone)]
pub struct Vec2<T: Num> {
    pub x: T,
    pub y: T,
}

impl<T: Num + Copy> Vec2<T> {
    pub fn new(x: T, y: T) -> Self {
        Self { x, y }
    }

    pub fn lerp(a: Vec2<T>, b: Vec2<T>, alpha: T) -> Self {
        let x = a.x + alpha * (b.x - a.x);
        let y = a.y + alpha * (b.y - a.y);
        Self { x, y }
    }
}

impl<T: Num> Add for Vec2<T> {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
        }
    }
}

impl<T: Num> Sub for Vec2<T> {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self {
            x: self.x - rhs.x,
            y: self.y - rhs.y,
        }
    }
}

impl<T: Num> Mul for Vec2<T> {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self::Output {
        Self {
            x: self.x * rhs.x,
            y: self.y * rhs.y,
        }
    }
}

impl<T: Num> Div for Vec2<T> {
    type Output = Self;

    fn div(self, rhs: Self) -> Self::Output {
        Self {
            x: self.x / rhs.x,
            y: self.y / rhs.y,
        }
    }
}

impl<T: NumAssign> AddAssign for Vec2<T> {
    fn add_assign(&mut self, rhs: Self) {
        self.x += rhs.x;
        self.y += rhs.y;
    }
}

impl<T: NumAssign> SubAssign for Vec2<T> {
    fn sub_assign(&mut self, rhs: Self) {
        self.x -= rhs.x;
        self.y -= rhs.y;
    }
}

impl<T: NumAssign> MulAssign for Vec2<T> {
    fn mul_assign(&mut self, rhs: Self) {
        self.x *= rhs.x;
        self.y *= rhs.y;
    }
}

impl<T: NumAssign> DivAssign for Vec2<T> {
    fn div_assign(&mut self, rhs: Self) {
        self.x /= rhs.x;
        self.y /= rhs.y;
    }
}

// Scalar
impl<T: Num + Copy> Add<T> for Vec2<T> {
    type Output = Self;

    fn add(self, rhs: T) -> Self::Output {
        Self {
            x: self.x + rhs,
            y: self.y + rhs,
        }
    }
}

impl<T: Num + Copy> Sub<T> for Vec2<T> {
    type Output = Self;

    fn sub(self, rhs: T) -> Self::Output {
        Self {
            x: self.x + rhs,
            y: self.y + rhs,
        }
    }
}

impl<T: Num + Copy> Mul<T> for Vec2<T> {
    type Output = Self;

    fn mul(self, rhs: T) -> Self::Output {
        Self {
            x: self.x * rhs,
            y: self.y * rhs,
        }
    }
}

impl<T: Num + Copy> Div<T> for Vec2<T> {
    type Output = Self;

    fn div(self, rhs: T) -> Self::Output {
        Self {
            x: self.x / rhs,
            y: self.y / rhs,
        }
    }
}

#[derive(Ord, PartialOrd, Eq, PartialEq, Debug, Default)]
pub struct Vec3<T: Default> {
    pub x: T,
    pub y: T,
    pub z: T,
}

impl<T: Default> Vec3<T> {
    pub fn new(x: T, y: T, z: T) -> Self {
        Self { x, y, z }
    }
}

#[derive(Ord, PartialOrd, Eq, PartialEq, Debug, Default)]
pub struct Vec4<T: Default> {
    pub x: T,
    pub y: T,
    pub z: T,
    pub w: T,
}

impl<T: Default> Vec4<T> {
    pub fn new(x: T, y: T, z: T, w: T) -> Self {
        Self { x, y, z, w }
    }
}

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

#[derive(Copy, Clone)]
pub struct Matrix<T: Default + Copy, const N: usize> {
    data: [[T; N]; N],
}

impl<T: Default + Copy, const N: usize> Matrix<T, N> {
    pub fn new(data: [[T; N]; N]) -> Self {
        Self { data }
    }
}

impl<T: Default + Copy, const N: usize> Default for Matrix<T, N> {
    fn default() -> Self {
        Self {
            data: [[T::default(); N]; N],
        }
    }
}

impl<T: Default + Copy, const N: usize> Index<usize> for Matrix<T, N> {
    type Output = [T; N];

    fn index(&self, index: usize) -> &Self::Output {
        &self.data[index]
    }
}

impl<T: Default + Copy, const N: usize> IndexMut<usize> for Matrix<T, N> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.data[index]
    }
}

pub type Vec2u32 = Vec2<u32>;
pub type Vec2i32 = Vec2<i32>;
pub type Vec2f32 = Vec2<f32>;

pub type Vec3u32 = Vec3<u32>;
pub type Vec3i32 = Vec3<i32>;
pub type Vec4f32 = Vec4<f32>;

pub type RectF32 = Rect<f32>;
pub type RectI32 = Rect<i32>;

pub type Matrix4f32 = Matrix<f32, 4>;
