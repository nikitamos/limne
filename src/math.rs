use std::{cell::RefCell, rc::Rc, sync::Arc};

#[derive(Clone)]
pub struct Tensor<T: Clone> {
    shape: Vec<usize>,
    stride: Vec<usize>,
    storage: Rc<RefCell<Vec<T>>>,
}

pub trait MulIter<T: Clone> {
    fn mul_all(self, default: T) -> T;
}

impl<'a, U, T> MulIter<T> for U
where
    U: Iterator<Item = T>,
    T: std::ops::Mul<T, Output = T> + Copy,
{
    fn mul_all(self, default: T) -> T {
        self.scan(default, |s, x| {
            *s = *s * x;
            Some(*s)
        })
        .last()
        .unwrap_or(default)
    }
}

impl<T> Tensor<T>
where
    T: Clone,
{
    pub fn new(shape: &[usize], default: T) -> Self
    where
        T: Clone,
    {
        let mut cap = 1usize;
        for i in shape {
            cap *= i;
        }
        let storage = Rc::new(RefCell::new(Vec::with_capacity(cap)));
        for _ in 0..cap {
            storage.borrow_mut().push(default.clone());
        }
        let stride: Vec<_> = shape.iter().map(|a| cap / a).collect();
        Tensor {
            shape: shape.to_owned(),
            stride,
            storage,
        }
    }
}

impl<T> std::ops::Add<&Tensor<T>> for Tensor<T>
where
    T: Clone,
{
    type Output = Tensor<T>;
    fn add(self, rhs: &Tensor<T>) -> Self::Output {
        assert_eq!(
            self.shape, rhs.shape,
            "trying to add tensors with distinct shapes"
        );
        let mut out = self.clone();
        for (i, b) in &mut out.shape.iter_mut().enumerate() {
            *b += self.shape[i];
        }
        out
    }
}

impl<T> std::ops::Mul<&Tensor<T>> for Tensor<T> where T: Clone {
    type Output = Tensor<T>;

    fn mul(self, rhs: &Tensor<T>) -> Self::Output {
        todo!()
    }
}