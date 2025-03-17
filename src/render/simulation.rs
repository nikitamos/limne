use core::slice;
use std::ops::{Deref, DerefMut};

use wgpu::{CommandBuffer, CommandEncoder, VertexBufferLayout};

use crate::math::vector::{NumVector3D, Vector3D};

pub trait AsBuffer {
  fn layout(&self) -> VertexBufferLayout<'static>;
  fn make_bytes(&self) -> &[u8];
}
pub trait VertexFormat {
  const ATTR: [wgpu::VertexAttribute; 1];
  fn vertex_format() -> &'static [wgpu::VertexAttribute] {
    &Self::ATTR
  }
}
struct ParticleVector<T: Copy>(Vec<NumVector3D<T>>);

impl<T: Copy> From<Vec<NumVector3D<T>>> for ParticleVector<T> {
  fn from(value: Vec<NumVector3D<T>>) -> Self {
    Self(value)
  }
}

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

impl<T: Copy> AsBuffer for ParticleVector<T>
where
  NumVector3D<T>: VertexFormat,
{
  fn layout(&self) -> VertexBufferLayout<'static> {
    VertexBufferLayout {
      array_stride: std::mem::size_of::<NumVector3D<T>>() as u64,
      step_mode: wgpu::VertexStepMode::Vertex,
      attributes: NumVector3D::<T>::vertex_format(),
    }
  }

  fn make_bytes(&self) -> &[u8] {
    let item_size = std::mem::size_of::<NumVector3D<T>>();
    unsafe { slice::from_raw_parts(self.as_slice().as_ptr().cast(), self.len() * item_size) }
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
  fn encoder_label<'a>(&self) -> Option<&'a str> {
    Some("Simulation encoder")
  }
  fn init_pipelines(&self, device: &wgpu::Device);
  fn run_passes(&self, encoder: CommandEncoder, view: &wgpu::TextureView) -> CommandBuffer;
}

pub trait SimulationFactory {
  fn new() -> Self;
  fn init_pipelines(self, device: &wgpu::Device) -> Self;
  fn build() -> Box<dyn Simulation>;
}

pub mod two_d {
  use rand::Rng;

  use super::*;

  pub struct DefaultSim {
    positions: ParticleVector<f32>,
  }

  pub struct DSFactory {
    positions: Option<ParticleVector<f32>>,
    // device: 
  }

  impl DefaultSim {
    pub fn new(count: usize, device: &wgpu::Device) -> Self {
      let mut positions: Vec<NumVector3D<f32>> = vec![Default::default(); count];
      let mut rng = rand::rng();
      for p in positions.iter_mut() {
        p.x = rng.sample(rand::distr::Uniform::new(0.0f32, 1.0f32).unwrap());
        p.y = rng.sample(rand::distr::Uniform::new(0.0f32, 1.0f32).unwrap());
        p.z = rng.sample(rand::distr::Uniform::new(0.0f32, 1.0f32).unwrap());
      }

      Self {
        positions: positions.into(),
      }
    }
  }

  impl Simulation for DefaultSim {
    fn step(&mut self, dt: f32) {
      todo!()
    }

    fn run_passes(
      &self,
      mut encoder: wgpu::CommandEncoder,
      view: &wgpu::TextureView,
    ) -> wgpu::CommandBuffer {
      {
        let pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
          label: None,
          color_attachments: &[],
          depth_stencil_attachment: None,
          timestamp_writes: None,
          occlusion_query_set: None,
        });
        
      }

      encoder.finish()
    }
    
    fn init_pipelines(&self, device: &wgpu::Device) {
        let bg_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: None,
            entries: &[
              wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: todo!(),
                ty: todo!(),
                count: todo!(),
            }
            ],
        });
    }
  }
}
