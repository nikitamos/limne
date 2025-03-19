use std::cell::{Ref, RefCell, RefMut};

struct Swapchain<T> {
  state: [RefCell<T>; 2],
  cur: usize,
}

impl<T: Clone> Swapchain<T> {
  pub fn init_with(state: T) -> Self {
    todo!()
  }
  pub fn cur(&self) -> Ref<'_, T> {
    self.state[self.cur].borrow()
  }
  pub fn cur_mut(&self) -> RefMut<'_, T> {
    self.state[self.cur].borrow_mut()
  }
  pub fn old(&self) -> Ref<'_, T> {
    self.state[1 - self.cur].borrow()
  }
  fn old_mut(&self) -> RefMut<'_, T> {
    self.state[1 - self.cur].borrow_mut()
  }
  pub fn swap(&mut self) -> (RefMut<'_, T>, Ref<'_, T>) {
    self.cur = 1 - self.cur;
    (self.cur_mut(), self.old())
  }
}
