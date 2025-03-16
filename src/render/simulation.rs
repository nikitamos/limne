use core::slice;
use std::ops::{Deref, DerefMut};

use wgpu::VertexBufferLayout;

use crate::math::vector::{Vector3D, NumVector3D};

pub trait AsBuffer {
  fn layout(&self) -> VertexBufferLayout<'static>;
  fn make_bytes(&self) -> &[u8];
}
pub trait VertexFormat {
  const ATTR: [wgpu::VertexAttribute;1 ];
  fn vertex_format() -> &'static [wgpu::VertexAttribute] {
    &Self::ATTR
  }
}
struct ParticleVector<T: Copy>(Vec<NumVector3D<T>>);

impl<T: Copy> Deref for ParticleVector<T> {
    type Target = Vec<NumVector3D<T>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl<T: Copy> DerefMut for ParticleVector<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<T: Copy> AsBuffer for ParticleVector<T> where NumVector3D<T>: VertexFormat {
    fn layout(&self) -> VertexBufferLayout<'static> {
        VertexBufferLayout {
            array_stride: std::mem::size_of::<NumVector3D<T>>() as u64,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: NumVector3D::<T>::vertex_format()
        }
    }

    fn make_bytes(&self) -> &[u8] {
        let item_size = std::mem::size_of::<NumVector3D<T>>();
        unsafe {slice::from_raw_parts( self.as_slice().as_ptr().cast(), self.len()*item_size)}
    }
}

impl VertexFormat for NumVector3D<f32> {
    const ATTR: [wgpu::VertexAttribute; 1] = [wgpu::VertexAttribute {
      offset: 0,
      format: wgpu::VertexFormat::Float32x3,
      shader_location: 0,
    }];
}

pub trait Simulation {
    fn step(&mut self, dt: f32);
    fn do_some_render(&self);
}

pub mod two_d {

}