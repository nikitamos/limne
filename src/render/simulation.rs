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

fn make_vec_buf<T>(v: &Vec<T>) -> &[u8] {
  unsafe { slice::from_raw_parts(v[..].as_ptr().cast(), v.len() * std::mem::size_of::<T>()) }
}

pub mod two_d {
  use core::f32;
  use std::num::NonZero;

  use rand::Rng;
  use wgpu::{
    util::BufferInitDescriptor, vertex_attr_array, MultisampleState, RenderPassColorAttachment,
    RenderPipelineDescriptor,
  };
  use winit::dpi::PhysicalSize;

  use super::*;

  #[repr(C)]
  #[derive(Default, Clone)]
  struct DefaultCell {
    pub velocity: NumVector3D<f32>,
    pub pressure: f32,
    pub count: f32,
  }

  pub struct DefaultSim {
    positions: ParticleVector<f32>,
    cells: Vec<DefaultCell>,
    pipeline: Option<wgpu::RenderPipeline>,
    instance_buf: Option<wgpu::Buffer>,
    cells_buf: Option<wgpu::Buffer>,
    celldims_buf: Option<wgpu::Buffer>,
    bind_group: Option<wgpu::BindGroup>,
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
      let x_cells = (2 * size.width).div_ceil(CELL_SIZE) as usize;
      let y_cells = (2 * size.height).div_ceil(CELL_SIZE) as usize;

      let mut cells = vec![DefaultCell::default(); x_cells * y_cells];
      const VEL_BOUND: f32 = 700.0;
      let distr = rand::distr::Uniform::new(0.0f32, f32::consts::TAU).unwrap();
      for c in cells.iter_mut() {
        let angle = rng.sample(distr);
        c.velocity.x = angle.cos() * VEL_BOUND;
        c.velocity.y = angle.sin() * VEL_BOUND;
      }

      let mut out = Self {
        positions: positions.into(),
        pipeline: None,
        bind_group: None,
        instance_buf: None,
        cells_buf: None,
        celldims_buf: None,
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
        (((pos.x + self.width) / (CELL_SIZE as f32)) as usize).clamp(0, self.x_cells - 1),
        (((pos.y + self.height) / (CELL_SIZE as f32)) as usize).clamp(0, self.y_cells - 1),
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
        pass.set_bind_group(1, &self.bind_group, &[]);
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
      let cell_bg_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("Cell binding layout"),
        entries: &[
          wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::all(),
            ty: wgpu::BindingType::Buffer {
              ty: wgpu::BufferBindingType::Storage { read_only: true },
              has_dynamic_offset: false,
              min_binding_size: None,
            },
            count: None,
          },
          wgpu::BindGroupLayoutEntry {
            binding: 1,
            visibility: wgpu::ShaderStages::all(),
            ty: wgpu::BindingType::Buffer {
              ty: wgpu::BufferBindingType::Storage { read_only: true },
              has_dynamic_offset: false,
              min_binding_size: None,
            },
            count: None,
          },
          // wgpu::BindGroupLayoutEntry {
          //   binding: 2,
          //   visibility: wgpu::ShaderStages::all(),
          //   ty: wgpu::BindingType::Buffer {
          //     ty: wgpu::BufferBindingType::Storage { read_only: true },
          //     has_dynamic_offset: false,
          //     min_binding_size: None,
          //   },
          //   count: None,
          // },
        ],
      });

      let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[global_layout, &cell_bg_layout],
            push_constant_ranges: &[],
        });

      // PARTICLE DRAWING (geometry)
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
            cull_mode: None,
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

      // CELLS
      let cell_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Cell Buffer"),
        size: make_vec_buf(&self.cells).len() as u64,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::STORAGE,
        mapped_at_creation: false,
      });
      let celldims_buf = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Cell dimension buffer"),
        size: 2 * std::mem::size_of::<u32>() as u64 + std::mem::size_of::<f32>() as u64,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::STORAGE,
        mapped_at_creation: false,
      });
      let cell_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("Cell bind group"),
        layout: &cell_bg_layout,
        entries: &[
          wgpu::BindGroupEntry {
            binding: 0,
            resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
              buffer: &cell_buffer,
              offset: 0,
              size: None,
            }),
          },
          wgpu::BindGroupEntry {
            binding: 1,
            resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
              buffer: &celldims_buf,
              offset: 0,
              size: None,
            }),
          },
          // wgpu::BindGroupEntry {
          //   binding: 2,
          //   resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
          //     buffer: &cell_buffer,
          //     offset: 2 * std::mem::size_of::<u32>() as u64,
          //     size: NonZero::new(std::mem::size_of::<f32>() as u64),
          //   }),
          // },
        ],
      });

      self.instance_buf = Some(instance_buf);
      self.pipeline = Some(pipeline);

      self.cells_buf = Some(cell_buffer);
      self.celldims_buf = Some(celldims_buf);
      self.bind_group = Some(cell_bg);
    }

    fn write_buffers(&self, queue: &wgpu::Queue) {
      queue.write_buffer(
        self.instance_buf.as_ref().unwrap(),
        0,
        self.positions.as_bytes_buffer(),
      );
      queue.write_buffer(
        self.cells_buf.as_ref().unwrap(),
        0,
        make_vec_buf(&self.cells),
      );
      let celldims = (self.x_cells as u32, self.y_cells as u32);
      // TODO: make sane implementation
      let a: Vec<_> = celldims
        .0
        .to_ne_bytes()
        .into_iter()
        .chain(celldims.1.to_ne_bytes().into_iter())
        .chain((CELL_SIZE as f32).to_ne_bytes().into_iter())
        .collect();

      queue.write_buffer(self.celldims_buf.as_ref().unwrap(), 0, &a);
    }
  }
}
