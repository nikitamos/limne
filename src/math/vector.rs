use std::{
    ops::{Add, Div, Mul, Sub},
    process::Output,
};

/// Vector in an orthonormal right-hand 3D basis
pub trait Vector3D<T>:
    Add<Self, Output = Self>
    + Sub<Self, Output = Self>
    + Mul<T, Output = Self>
    + Div<T, Output = Self>
    + Sized
    + Clone
{
    fn new(x: T, y: T, z: T) -> Self;
    fn dot(&self, rhs: &Self) -> T;
    fn cross(&self, rhs: &Self) -> Self;
    fn length_squared(&self) -> T {
      self.dot(self)
    }
    fn x(&self) -> T;
    fn y(&self) -> T;
    fn z(&self) -> T;
}

#[derive(Clone)]
struct NumVector3D<T: Copy> {
    x: T,
    y: T,
    z: T,
}

impl<T: Copy + Div<Output = T>> Div<T> for NumVector3D<T> {
    type Output = Self;

    fn div(self, rhs: T) -> Self::Output {
        NumVector3D {
            x: self.x / rhs,
            y: self.y / rhs,
            z: self.z / rhs,
        }
    }
}

impl<T: Copy + Mul<Output = T>> Mul<T> for NumVector3D<T> {
    type Output = Self;

    fn mul(self, rhs: T) -> Self::Output {
        Self {
            x: self.x * rhs,
            y: self.y * rhs,
            z: self.z * rhs,
        }
    }
}

impl<T: Copy + Add<Output = T>> Add<Self> for NumVector3D<T> {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
            z: self.z + rhs.z,
        }
    }
}
impl<T: Copy + Sub<Output = T>> Sub<Self> for NumVector3D<T> {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self {
            x: self.x - rhs.x,
            y: self.y - rhs.y,
            z: self.z - rhs.z,
        }
    }
}

impl<T: Copy> Vector3D<T> for NumVector3D<T>
where
    T: Mul<Output = T> + Div<Output = T> + Add<Output = T> + Sub<Output = T>,
{
    fn new(x: T, y: T, z: T) -> Self {
        Self {x, y, z}
    }

    fn dot(&self, rhs: &Self) -> T {
        self.x * rhs.x + self.y * rhs.y + self.z * rhs.z
    }

    fn cross(&self, rhs: &Self) -> Self {
        Self {
            x: self.y * rhs.z - self.z * rhs.y,
            y: self.z * rhs.x - self.x * rhs.z,
            z: self.x * rhs.y - self.y * rhs.x
        }
    }

    fn x(&self) -> T {
        self.x
    }

    fn y(&self) -> T {
        self.y
    }

    fn z(&self) -> T {
        self.z
    }
}

struct SimdVectorArray {
  
}