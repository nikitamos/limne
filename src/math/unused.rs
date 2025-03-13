#[derive(Clone)]
pub struct CpuTensor<T: Clone> {
    shape: Vec<usize>,
    stride: Vec<usize>,
    storage: Rc<RefCell<Vec<T>>>,
}

pub trait MulIter<T: Clone> {
    fn mul_all(self, default: T) -> T;
}

impl<T> CpuTensor<T>
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
        CpuTensor {
            shape: shape.to_owned(),
            stride: calculate_strides(shape, Some(cap)).0,
            storage,
        }
    }

    pub fn convolute(&mut self, i1: usize, i2: usize) -> &Self {
        todo!()
    }

    #[must_use]
    pub fn convolute_copy(&self, i1: usize, i2: usize) -> Self {
        let mut t = self.clone().to_owned();
        t.convolute(i1, i2);
        t
    }
}

#[inline]
#[must_use]
fn calculate_strides(shape: &[usize], cap: Option<usize>) -> (Vec<usize>, usize) {
    let cap = cap.unwrap_or_else(|| {
        let mut out = 1;
        for i in shape {
            out *= *i;
        }
        out
    });
    (shape.iter().map(|a| cap / a).collect(), cap)
}

impl<T> std::ops::Add<&CpuTensor<T>> for CpuTensor<T>
where
    T: Clone,
{
    type Output = CpuTensor<T>;
    fn add(self, rhs: &CpuTensor<T>) -> Self::Output {
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

impl<T: Clone> Index<&[usize]> for CpuTensor<T>
where
// Idx: Index<usize, Output = usize> + IntoIterator,
{
    type Output = T;

    fn index(&self, index: &[usize]) -> &Self::Output {
        assert_eq!(index.len(), self.shape.len());

        todo!()
    }
}

// struct TensorIndexIterator<'a> {
//     num: usize,
//     current: Box<[usize]>,
//     shape: &'a Vec<usize>
// }

// impl<'a> TensorIndexIterator<'a> {
//     fn create(shape: &'a Vec<usize>) -> Self {

//       TensorIndexIterator {
//         num: shape.iter().sum(),
//         current: ,
//         shape,
//       }
//     }
// }

// impl<'a> Iterator for TensorIndexIterator<'a> {
//     type Item = &'a Vec<usize>;

//     fn next(&mut self) -> Option<Self::Item> {
//         todo!()
//     }
// }

impl<T> std::ops::Mul<&CpuTensor<T>> for CpuTensor<T>
where
    T: Clone,
{
    type Output = CpuTensor<T>;

    fn mul(self, rhs: &CpuTensor<T>) -> Self::Output {
        let shape: Vec<_> = self
            .shape
            .iter()
            .copied()
            .chain(rhs.shape.iter().copied())
            .collect();
        let (strides, cap) = calculate_strides(shape.as_slice(), None);

        let shape_iter = shape.iter().copied().map(|x| 0..x);

        todo!()
    }
}
