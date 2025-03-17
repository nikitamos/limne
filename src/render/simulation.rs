use core::slice;
use std::ops::{Deref, DerefMut};

use wgpu::{
  core::device::queue, util::DeviceExt, CommandBuffer, CommandEncoder, VertexBufferLayout,
};

use crate::math::vector::{NumVector3D, Vector3D};

pub trait AsBuffer {
  fn as_bytes_buffer(&self) -> &[u8];
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
  fn as_bytes_buffer(&self) -> &[u8] {
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
  fn init_pipelines(&mut self, device: &wgpu::Device, format: wgpu::TextureFormat);
  fn run_passes(&self, encoder: CommandEncoder, view: &wgpu::TextureView) -> CommandBuffer;
  fn write_buffers(&self, queue: &wgpu::Queue);
}

pub mod two_d {
  use rand::Rng;
  use wgpu::{
    util::BufferInitDescriptor, vertex_attr_array, MultisampleState, RenderPassColorAttachment,
    RenderPipelineDescriptor,
  };

  use super::*;

  pub struct DefaultSim {
    positions: ParticleVector<f32>,
    pipeline: Option<wgpu::RenderPipeline>,
    instance_buf: Option<wgpu::Buffer>,
  }

  impl DefaultSim {
    pub fn new(count: usize, device: &wgpu::Device) -> Self {
      let mut positions: Vec<NumVector3D<f32>> = vec![Default::default(); count];
      let mut rng = rand::rng();
      if count == 2 {
        positions[0] = NumVector3D {
          x: 0.5,
          y: 0.5,
          z: 0.0,
        };
        positions[1] = NumVector3D {
          x: -0.5,
          y: -0.5,
          z: 0.0,
        };
      } else {
        for p in positions.iter_mut() {
        p.x = rng.sample(rand::distr::Uniform::new(-1.0f32, 1.0f32).unwrap());
        p.y = rng.sample(rand::distr::Uniform::new(-1.0f32, 1.0f32).unwrap());
        p.z = rng.sample(rand::distr::Uniform::new(-1.0f32, 1.0f32).unwrap());
      }}

      Self {
        positions: positions.into(),
        pipeline: None,
        instance_buf: None
      }
    }
  }

  const DEFSIM_BUFFER_LAYOUT: VertexBufferLayout = VertexBufferLayout {
    array_stride: 3 * std::mem::size_of::<f32>() as u64,
    step_mode: wgpu::VertexStepMode::Instance,
    attributes: &vertex_attr_array![0 => Float32x3],
  };

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
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
          label: None,
          color_attachments: &[ Some (RenderPassColorAttachment {
            view,
            resolve_target: None,
            ops: wgpu::Operations { load: wgpu::LoadOp::Load, store: wgpu::StoreOp::Store },
        })],
          depth_stencil_attachment: None,
          timestamp_writes: None,
          occlusion_query_set: None,
        });
        pass.set_pipeline(self.pipeline.as_ref().unwrap());
        pass.set_vertex_buffer(0, self.instance_buf.as_ref().unwrap().slice(..));
        pass.draw(0..3, 0..(self.positions.len() as u32));
      }

      encoder.finish()
    }

    fn init_pipelines(&mut self, device: &wgpu::Device, format: wgpu::TextureFormat) {
      let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[],
            push_constant_ranges: &[],
        });

      let instance_buf = device.create_buffer_init(&BufferInitDescriptor {
        label: Some("Instance buffer"),
        contents: self.positions.as_bytes_buffer(),
        usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
      });

      let module = device.create_shader_module(wgpu::include_wgsl!("shaders/2d-basic.wgsl"));

      let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
        label: Some("Default simulation"),
        layout: Some(&layout),
        vertex: wgpu::VertexState {
          module: &module,
          entry_point: Some("vs_main"),
          compilation_options: Default::default(),
          buffers: &[
            DEFSIM_BUFFER_LAYOUT
          ],
        },
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            strip_index_format: None,
            front_face: wgpu::FrontFace::Ccw,
            cull_mode: Some(wgpu::Face::Back),
            unclipped_depth: false,
            polygon_mode: wgpu::PolygonMode::Fill,
            conservative: false,
        },
        depth_stencil: None,
        multisample: MultisampleState {
          count: 1,
          mask: !0,
          alpha_to_coverage_enabled: false,
        },
        fragment: Some (wgpu::FragmentState {
            module: &module,
            entry_point: Some("fs_main"),
            compilation_options: Default::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format,
                blend: Some(wgpu::BlendState::REPLACE),
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        multiview: None,
        cache: None,
      });
      self.instance_buf = Some(instance_buf);
      self.pipeline = Some(pipeline);
    }

    fn write_buffers(&self, queue: &wgpu::Queue) {
      queue.write_buffer(
        self.instance_buf.as_ref().unwrap(),
        0,
        self.positions.as_bytes_buffer(),
      );
    }
  }
}
