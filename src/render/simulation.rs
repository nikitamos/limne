use core::slice;
use std::ops::{Deref, DerefMut};

use wgpu::{util::DeviceExt, CommandBuffer, CommandEncoder, VertexBufferLayout};

use crate::math::vector::NumVector3D;

pub trait AsBuffer {
  fn as_bytes_buffer(&self) -> &[u8];
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

impl<T: Copy> AsBuffer for ParticleVector<T> {
  fn as_bytes_buffer(&self) -> &[u8] {
    let item_size = std::mem::size_of::<NumVector3D<T>>();
    unsafe { slice::from_raw_parts(self.as_slice().as_ptr().cast(), self.len() * item_size) }
  }
}

impl<const N: usize> AsBuffer for [f32; N] {
  fn as_bytes_buffer(&self) -> &[u8] {
    unsafe { slice::from_raw_parts(self.as_ptr().cast(), N * std::mem::size_of::<f32>()) }
  }
}

pub trait Simulation {
  fn step(&mut self, dt: f32);
  fn encoder_label<'a>(&self) -> Option<&'a str> {
    Some("Simulation encoder")
  }
  fn init_pipelines(
    &mut self,
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    global_layout: &wgpu::BindGroupLayout,
  );
  fn run_passes(
    &self,
    encoder: CommandEncoder,
    global_bind_group: &wgpu::BindGroup,
    view: &wgpu::TextureView,
  ) -> CommandBuffer;
  fn write_buffers(&self, queue: &wgpu::Queue);
}

pub mod two_d {
  use std::collections::HashMap;

  use rand::Rng;
  use wgpu::{
    util::BufferInitDescriptor, vertex_attr_array, Color, MultisampleState,
    RenderPassColorAttachment, RenderPipelineDescriptor,
  };
  use winit::dpi::PhysicalSize;

  use super::*;

  #[derive(Default, Clone)]
  struct DefaultCell {
    pub density: f32,
    pub velocity: NumVector3D<f32>,
    pub count: u32,
  }

  pub struct DefaultSim {
    positions: ParticleVector<f32>,
    cells: Vec<DefaultCell>,
    pipeline: Option<wgpu::RenderPipeline>,
    instance_buf: Option<wgpu::Buffer>,
    x_cells: usize,
    y_cells: usize,
    height: f32,
    width: f32,
  }

  const CELL_SIZE: u32 = 50;
  impl DefaultSim {
    pub fn new(count: usize, device: &wgpu::Device, size: PhysicalSize<u32>) -> Self {
      let mut positions: Vec<NumVector3D<f32>> = vec![Default::default(); count];
      let mut rng = rand::rng();
      let width = size.width as f32;
      let height = size.height as f32;

      for p in positions.iter_mut() {
        p.x = rng.sample(rand::distr::Uniform::new(-width, width).unwrap());
        p.y = rng.sample(rand::distr::Uniform::new(-height, height).unwrap());
      }

      // Create cells
      let x_cells = size.width.div_ceil(CELL_SIZE) as usize;
      let y_cells = size.width.div_ceil(CELL_SIZE) as usize;

      let mut cells = vec![DefaultCell::default(); x_cells * y_cells];
      const VEL_BOUND: f32 = 700.0;
      for c in cells.iter_mut() {
        c.velocity.x = rng.sample(rand::distr::Uniform::new(-VEL_BOUND, VEL_BOUND).unwrap());
        c.velocity.y = rng.sample(rand::distr::Uniform::new(-VEL_BOUND, VEL_BOUND).unwrap());
      }

      let mut out = Self {
        positions: positions.into(),
        pipeline: None,
        instance_buf: None,
        x_cells,
        y_cells,
        cells,
        height,
        width
      };

      for i in 0..x_cells {
        out.cells[i].velocity = Default::default();
        out.cells[(y_cells - 1) * x_cells + i] = Default::default();
      }
      for j in 1..(y_cells - 1) {
        out.cells[j * (x_cells - 1)] = Default::default();
        out.cells[(j * x_cells) - 1] = Default::default();
      }

      out
    }
    fn get_cell(&self, pos: NumVector3D<f32>) -> (DefaultCell, (usize, usize)) {
      let idx = (
        (((pos.x + self.width / 2.0) / (CELL_SIZE as f32)) as usize).clamp(0, self.x_cells - 1),
        (((pos.y + self.height / 2.0) / (CELL_SIZE as f32)) as usize).clamp(0, self.y_cells - 1),
      );
      (
        self
          .cells
          .get(idx.1 * self.x_cells + idx.0)
          .unwrap()
          .clone(),
        idx,
      )
    }
    fn cell_mut(&mut self, pos: NumVector3D<f32>) -> (&mut DefaultCell, (usize, usize)) {
      let idx = (
        (((pos.x + self.width / 2.0) / (CELL_SIZE as f32)) as usize)
          .clamp(0, self.positions.len() - 1),
        (((pos.y + self.height / 2.0) / (CELL_SIZE as f32)) as usize)
          .clamp(0, self.positions.len() - 1),
      );
      (
        self.cells.get_mut(idx.1 + self.x_cells * idx.0).unwrap(),
        idx,
      )
    }
  }

  const DEFSIM_BUFFER_LAYOUT: VertexBufferLayout = VertexBufferLayout {
    array_stride: 3 * std::mem::size_of::<f32>() as u64,
    step_mode: wgpu::VertexStepMode::Instance,
    attributes: &vertex_attr_array![0 => Float32x3],
  };

  impl Simulation for DefaultSim {
    fn step(&mut self, dt: f32) {
      // for now we consider that each particle velocity is just deictated by the containing cell
      let len = self.positions.len();
      for i in 0..len {
        let c = self.get_cell(self.positions[i]).0;
        self.positions[i] += c.velocity * dt;
      }
    }

    fn run_passes(
      &self,
      mut encoder: wgpu::CommandEncoder,
      global_bind_group: &wgpu::BindGroup,
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
        pass.set_bind_group(0, global_bind_group, &[]);
        pass.draw(0..3, 0..(self.positions.len() as u32));
      }
      println!("DefSim pass!");
      encoder.finish()
    }

    fn init_pipelines(
      &mut self,
      device: &wgpu::Device,
      format: wgpu::TextureFormat,
      global_layout: &wgpu::BindGroupLayout,
    ) {
      let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[global_layout],
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
