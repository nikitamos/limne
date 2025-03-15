use std::{
    default::{self, Default},
    ops::{Index, IndexMut},
};

pub struct TensorIndexer {
    current_idx: Vec<usize>,
    shape: Vec<usize>,
    done: bool,
}

impl TensorIndexer {
    pub fn create<'a, T: Clone, Ts: Tensor<'a, T>>(t: &Ts) -> TensorIndexer
    where
        Self: Sized,
    {
        TensorIndexer {
            current_idx: vec![0; t.rank()],
            shape: t.shape(),
            done: false,
        }
    }
    pub fn next_idx(&mut self) -> Option<Vec<usize>> {
        if self.done {
            None
        } else if self.shape.len() == 1 && self.shape[0] == 1 {
            if self.current_idx[0] == 0 {
                self.current_idx[0] += 1;
                Some(vec![0])
            } else {
                None
            }
        } else {
            let out = self.current_idx.clone();
            let mut carry = 1;
            for i in (0..self.current_idx.len()).rev() {
                self.current_idx[i] += carry;
                if self.current_idx[i] == self.shape[i] {
                    carry = 1;
                    self.current_idx[i] = 0;
                } else {
                    carry = 0;
                    break;
                }
            }
            if carry == 1 {
                self.done = true;
            }
            Some(out)
        }
    }
}

pub trait Tensor<'a, T>: Index<&'a [usize], Output = T> + Clone
where
    T: Clone,
{
    fn rank(&self) -> usize;
    fn shape(&self) -> Vec<usize>;
    fn tensor_mul(&self, rhs: &Self) -> Self;
    fn convolve(self, i1: usize, i2: usize) -> Self;
    fn indexer(&self) -> TensorIndexer {
        TensorIndexer::create(self)
    }
}

#[derive(Copy, Clone, Default)]
struct Vec3<T>(T, T, T);

#[derive(Clone, Debug)]
pub struct TensorND<T, const N: usize>
where
    T: Clone,
{
    rank: usize,
    data: Vec<T>,
}

impl<T: Clone, const N: usize> Index<&[usize]> for TensorND<T, N> {
    type Output = T;

    fn index(&self, index: &[usize]) -> &T {
        &self.data[self.index_offset(index)]
    }
}

impl<T: Clone, const N: usize> IndexMut<&[usize]> for TensorND<T, N> {
    fn index_mut(&mut self, index: &[usize]) -> &mut Self::Output {
        let o = self.index_offset(index);
        &mut self.data[o]
    }
}

pub type Tensor3D<T> = TensorND<T, 3>;

impl<T: Clone + Default, const N: usize> TensorND<T, N> {
    pub fn default_values(rank: usize) -> Self {
        Self {
            rank,
            data: vec![Default::default(); N.pow((rank) as u32)],
        }
    }
}

impl<T: Clone, const N: usize> TensorND<T, N> {
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

pub struct NDINdexer {}

impl<T, const N: usize> Tensor<'_, T> for TensorND<T, N>
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
