use std::{
    default::Default,
    ops::{Add, Index, IndexMut},
};

pub trait Tensor<'a, T>: Index<&'a [usize], Output = T> + Clone
where
    T: Clone,
{
    fn rank(&self) -> usize;
    fn shape(&self) -> Vec<usize>;
    fn tensor_mul(&self, rhs: &Self) -> Self;
    fn convolve(self, i1: usize, i2: usize) -> Self;
}

#[derive(Copy, Clone)]
struct Vec3<T: Clone>(T, T, T);

impl<T: Default + Clone> Default for Vec3<T> {
    fn default() -> Self {
        Self(Default::default(), Default::default(), Default::default())
    }
}

#[derive(Clone)]
pub struct CpuTensorND<T, const N: usize>
where
    T: Clone,
{
    rank: usize,
    data: Vec<T>,
}

impl<T: Clone, const N: usize> Index<&[usize]> for CpuTensorND<T, N> {
    type Output = T;

    fn index(&self, index: &[usize]) -> &T {
        &self.data[self.index_offset(index)]
    }
}

impl<T: Clone, const N: usize> IndexMut<&[usize]> for CpuTensorND<T, N> {
    fn index_mut(&mut self, index: &[usize]) -> &mut Self::Output {
        let o = self.index_offset(index);
        &mut self.data[o]
    }
}

pub type CpuTensor3D<T> = CpuTensorND<T, 3>;

impl<T: Clone + Default, const N: usize> CpuTensorND<T, N> {
    fn default_values(rank: usize) -> Self {
        Self {
            rank,
            data: vec![Default::default(); N.pow((rank) as u32)],
        }
    }
}

impl<T: Clone, const N: usize> CpuTensorND<T, N> {
    fn index_offset(&self, index: &[usize]) -> usize {
        if self.rank == 0 {
            assert_eq!(index.len(), 1);
            assert_eq!(index[0], 0);
            return 0;
        } else {
            assert_eq!(index.len(), self.rank);
        }
        let mut offset = 0;
        let mut stride = N.pow((self.rank - 1) as u32);
        for i in 0..index.len() {
            offset += stride * i;
            stride /= self.rank;
        }
        offset
    }
}

impl<T, const N: usize> Tensor<'_, T> for CpuTensorND<T, N>
where
    T: Clone,
{
    fn rank(&self) -> usize {
        self.rank
    }

    fn shape(&self) -> Vec<usize> {
        if self.rank == 0 {
            vec![1]
        } else {
            vec![3; self.rank]
        }
    }

    fn tensor_mul(&self, rhs: &Self) -> Self {
        todo!()
    }

    fn convolve(self, i1: usize, i2: usize) -> Self {
        todo!()
    }
}
