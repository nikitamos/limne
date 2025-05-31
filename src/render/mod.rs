use core::slice;

use cgmath::{Matrix, Matrix4};

pub mod application;
pub mod camera;
pub mod render_target;
pub mod state;
pub mod swapchain;
pub mod targets;
pub mod texture_provider;

pub trait AsBuffer {
  fn as_bytes_buffer(&self) -> &[u8];
}

impl AsBuffer for Matrix4<f32> {
  fn as_bytes_buffer(&self) -> &[u8] {
    unsafe { slice::from_raw_parts(self.as_ptr().cast(), std::mem::size_of::<Matrix4<f32>>()) }
  }
}

impl<const N: usize> AsBuffer for [f32; N] {
  fn as_bytes_buffer(&self) -> &[u8] {
    unsafe { slice::from_raw_parts(self.as_ptr().cast(), N * std::mem::size_of::<f32>()) }
  }
}

impl<const N: usize> AsBuffer for [u16; N] {
  fn as_bytes_buffer(&self) -> &[u8] {
    unsafe { slice::from_raw_parts(self.as_ptr().cast(), N * std::mem::size_of::<u16>()) }
  }
}

impl AsBuffer for &[f32] {
  fn as_bytes_buffer(&self) -> &[u8] {
    unsafe {
      slice::from_raw_parts(
        self.as_ptr().cast(),
        self.len() * std::mem::size_of::<f32>(),
      )
    }
  }
}
