use std::ops::{Add, Mul, Sub};

/// Vector in an orthonormal right-hand 3D basis
pub trait Vector3D: Add<Self, Output = Self> + Sub<Self, Output = Self> + Sized + Clone
// where
    // ,
{
    type T: Mul<Self, Output = T> + Add<Self, Output = Self::Self::T>;
    fn new(x: T, y: T, z: T);
    fn dot(&self, rhs: &Self) -> T;
    fn cross(&self, rhs: &Self) -> T;
    fn length_squared(&self) -> T;
    fn x(&self) -> T;
    fn y(&self) -> T;
    fn z(&self) -> T;
}

struct NumVector3D<T: Copy> {
    x: T,
    y: T,
    z: T
}

impl <T> Mul<NumVector3D<T>> for T where T: Copy {
    type Output = NumVector3D<T>;

    fn mul(self, rhs: NumVector3D<T>) -> Self::Output {
        return 
    }
}