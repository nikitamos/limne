use core::slice;
use std::ops::{Deref, DerefMut};

use crate::math::vector::NumVector3D;
use two_d::DefaultCell;
use wgpu::{CommandBuffer, CommandEncoder, VertexBufferLayout};

pub trait AsBuffer {
  fn as_bytes_buffer(&self) -> &[u8];
}

#[rustfmt::skip]
const SQUARE: [f32; 12] = [
  0., 0.,
  0., 1.,
  1., 1.,
  1., 1.,
  0., 0.,
  1., 0.
];

#[derive(Clone)]
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

#[derive(Clone, Copy)]
pub struct SimulationRegenOptions {
  pub size: f32,
  pub vmin: f32,
  pub vmax: f32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct SimulationParams {
  pub k: f32,
  pub m0: f32,
}

impl Default for SimulationParams {
  fn default() -> Self {
    Self { k: 0.3, m0: 0.01 }
  }
}

impl AsBuffer for SimulationParams {
  fn as_bytes_buffer(&self) -> &[u8] {
    unsafe {
      slice::from_raw_parts(
        std::ptr::from_ref(self).cast(),
        std::mem::size_of::<SimulationParams>(),
      )
    }
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
  fn reinit_pipelines(
    &mut self,
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    global_layout: &wgpu::BindGroupLayout,
  ) {
    self.init_pipelines(device, format, global_layout);
  }
  fn run_passes(
    &mut self,
    encoder: CommandEncoder,
    global_bind_group: &wgpu::BindGroup,
    view: &wgpu::TextureView,
  ) -> CommandBuffer;
  fn write_buffers(&self, queue: &wgpu::Queue);
  fn on_surface_resized(&mut self, size: egui::Vec2, device: &wgpu::Device);
}

fn make_vec_buf<T>(v: &Vec<T>) -> &[u8] {
  unsafe { slice::from_raw_parts(v[..].as_ptr().cast(), v.len() * std::mem::size_of::<T>()) }
}

impl AsBuffer for Vec<DefaultCell> {
  fn as_bytes_buffer(&self) -> &[u8] {
    make_vec_buf(self)
  }
}

pub mod two_d {
  use core::f32;
  use rand::Rng;
  use rayon::prelude::*;
  use wgpu::{
    util::{BufferInitDescriptor, DeviceExt},
    vertex_attr_array, BufferUsages, FragmentState, MultisampleState, PrimitiveState,
    RenderPassColorAttachment, RenderPipelineDescriptor, ShaderStages,
  };

  use crate::render::swapchain::{SwapBuffers, SwapBuffersDescriptor};

  use super::*;

  #[repr(C)]
  #[derive(Clone)]
  pub(super) struct DefaultCell {
    pub velocity: NumVector3D<f32>,
    pub pressure: f32,
    pub density: f32,
  }

  impl Default for DefaultCell {
    fn default() -> Self {
      Self {
        velocity: Default::default(),
        pressure: Default::default(),
        density: 2.0,
      }
    }
  }

  pub struct DefaultSim {
    positions: SwapBuffers<ParticleVector<f32>>,
    cells: SwapBuffers<Vec<DefaultCell>>,
    move_pipeline: Option<wgpu::ComputePipeline>,
    mass_consv_pipeline: Option<wgpu::ComputePipeline>,
    pipeline: Option<wgpu::RenderPipeline>,
    density_pipeline: Option<wgpu::RenderPipeline>,
    square_buffer: Option<wgpu::Buffer>,
    grid_buf: Option<wgpu::Buffer>,
    grid_bg: Option<wgpu::BindGroup>,
    params_buf: Option<wgpu::Buffer>,
    params_bg: Option<wgpu::BindGroup>,
    x_cells: usize,
    y_cells: usize,
    height: f32,
    width: f32,
    opts: SimulationRegenOptions,
    params: SimulationParams,
  }

  impl DefaultSim {
    pub fn create_fully_initialized(
      count: usize,
      device: &wgpu::Device,
      size: egui::Vec2,
      format: wgpu::TextureFormat,
      global_layout: &wgpu::BindGroupLayout,
      opts: SimulationRegenOptions,
    ) -> Self {
      let mut out = Self::new(count, device, size, opts);
      out.init_pipelines(device, format, global_layout);
      out
    }
    pub fn new(
      count: usize,
      device: &wgpu::Device,
      size: egui::Vec2,
      opts: SimulationRegenOptions,
    ) -> Self {
      let mut positions: Vec<NumVector3D<f32>> = vec![Default::default(); count];
      let mut rng = rand::rng();
      let width = size.x;
      let height = size.y;

      for p in positions.iter_mut() {
        p.x = rng.sample(rand::distr::Uniform::new(0.0, width).unwrap());
        p.y = rng.sample(rand::distr::Uniform::new(0.0, height).unwrap());
      }

      // Create cells
      let x_cells = (2 * size.x as u32).div_ceil(opts.size as u32) as usize;
      let y_cells = (2 * size.y as u32).div_ceil(opts.size as u32) as usize;

      let mut cells = vec![DefaultCell::default(); x_cells * y_cells];
      const VEL_BOUND: f32 = 70.0;
      let distr = rand::distr::Uniform::new(0.0f32, f32::consts::TAU).unwrap();
      for c in cells.iter_mut() {
        let angle = rng.sample(distr);
        c.velocity.x = angle.cos() * VEL_BOUND;
        c.velocity.y = angle.sin() * VEL_BOUND;
      }

      for i in 0..y_cells {
        cells[i].velocity.y = 0.0;
        cells[(y_cells - 1) * x_cells - 1].velocity.y = 0.0;
      }
      for j in 0..x_cells {
        {
          cells[y_cells * j].velocity.x = 0.;
          cells[y_cells * (j + 1) - 1].velocity.x = 0.;
        }
      }

      let out = Self {
        positions: SwapBuffers::init_with(
          positions.into(),
          &device,
          SwapBuffersDescriptor {
            usage: wgpu::BufferUsages::STORAGE
              | wgpu::BufferUsages::COPY_DST
              | wgpu::BufferUsages::COPY_SRC
              | wgpu::BufferUsages::VERTEX,
            visibility: ShaderStages::all(),
            ty: wgpu::BufferBindingType::Storage { read_only: false },
            has_dynamic_offset: false,
          },
        ),
        pipeline: None,
        move_pipeline: None,
        mass_consv_pipeline: None,
        density_pipeline: None,
        square_buffer: None,
        grid_bg: None,
        grid_buf: None,
        params_buf: None,
        params_bg: None,
        x_cells,
        y_cells,
        cells: SwapBuffers::init_with(
          cells,
          device,
          SwapBuffersDescriptor {
            usage: wgpu::BufferUsages::STORAGE
              | wgpu::BufferUsages::COPY_DST
              | wgpu::BufferUsages::COPY_SRC,
            visibility: ShaderStages::all(),
            ty: wgpu::BufferBindingType::Storage { read_only: false },
            has_dynamic_offset: false,
          },
        ),
        height,
        width,
        opts,
        params: Default::default()
      };

      out
    }

    pub fn render_into_pass(
      &self,
      global_bind_group: &wgpu::BindGroup,
      pass: &mut wgpu::RenderPass<'_>,
    ) {
      pass.set_pipeline(self.density_pipeline.as_ref().unwrap());
      self.setup_groups_for_render(global_bind_group, pass);
      pass.set_vertex_buffer(0, self.square_buffer.as_ref().unwrap().slice(..));
      pass.draw(0..6, 0..(self.x_cells * self.y_cells) as u32);

      pass.set_pipeline(self.pipeline.as_ref().unwrap());
      pass.set_vertex_buffer(0, self.positions.cur_buf().slice(..));
      self.setup_groups_for_render(global_bind_group, pass);
      pass.draw(0..3, 0..(self.positions.cur().len() as u32));
    }

    fn setup_groups_for_render(
      &self,
      global_bind_group: &wgpu::BindGroup,
      pass: &mut wgpu::RenderPass<'_>,
    ) {
      pass.set_bind_group(0, global_bind_group, &[]);
      pass.set_bind_group(1, &self.grid_bg, &[]);
      pass.set_bind_group(2, self.cells.cur_group(), &[]);
    }

    pub fn compute(&mut self, encoder: &mut CommandEncoder, global_bind_group: &wgpu::BindGroup) {
      self.positions.swap(encoder);
      self.cells.swap(encoder);
      let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
        label: Some("DefSim COMPUTE pass"),
        timestamp_writes: None,
      });
      self.setup_groups_for_compute(global_bind_group, &mut pass);

      pass.set_pipeline(self.mass_consv_pipeline.as_ref().unwrap());
      pass.dispatch_workgroups(self.x_cells as u32, self.y_cells as u32, 1);

      pass.set_pipeline(self.move_pipeline.as_ref().unwrap());
      pass.dispatch_workgroups(self.positions.cur().len() as u32, 1, 1);
    }

    fn setup_groups_for_compute(
      &mut self,
      global_bind_group: &wgpu::BindGroup,
      pass: &mut wgpu::ComputePass<'_>,
    ) {
      pass.set_bind_group(0, global_bind_group, &[]);
      pass.set_bind_group(1, self.grid_bg.as_ref().unwrap(), &[]);
      pass.set_bind_group(2, self.positions.cur_group(), &[]);
      pass.set_bind_group(3, self.cells.cur_group(), &[]);
      pass.set_bind_group(4, self.params_bg.as_ref().unwrap(), &[]);
    }

    pub fn regenerate_grid(&mut self, device: &wgpu::Device, opts: SimulationRegenOptions) {
      self.opts = opts;
      let x_cells = (self.width / opts.size).ceil() as usize;
      let y_cells = (self.height / opts.size).ceil() as usize;
      println!("Cell count: {}", x_cells * y_cells);

      let cells = vec![DefaultCell::default(); x_cells * y_cells];
      let direction = rand::distr::Uniform::new(0.0f32, f32::consts::TAU).unwrap();
      let vel_distr = rand::distr::Uniform::new(opts.vmin, opts.vmax).unwrap();
      let density = rand::distr::Uniform::new(0.0, 3.0).unwrap();

      self.cells.reset(
        cells
          .into_par_iter()
          .map(|mut c| {
            let mut rng = rand::rng();
            let v = rng.sample(vel_distr);
            let angle = rng.sample(direction);
            c.velocity.x = angle.cos() * v;
            c.velocity.y = angle.sin() * v;
            c.density = rng.sample(density);
            c
          })
          .collect(),
        device,
      );

      self.x_cells = x_cells;
      self.y_cells = y_cells;
    }

    pub fn regenerate_positions(&mut self, device: &wgpu::Device) {
      let x_distr = rand::distr::Uniform::new(0.0, self.width).unwrap();
      let y_distr = rand::distr::Uniform::new(0.0, self.height).unwrap();
      self.positions.reset(
        self
          .positions
          .cur()
          .0
          .clone()
          .into_iter()
          .map(|mut p| {
            let mut rng = rand::rng();
            p.x = rng.sample(x_distr);
            p.y = rng.sample(y_distr);
            p
          })
          .collect::<Vec<_>>()
          .into(),
        device,
      );
    }
    pub fn set_params(&mut self, params: SimulationParams) {
      self.params = params;
    }
  }

  const DEFSIM_BUFFER_LAYOUT: VertexBufferLayout = VertexBufferLayout {
    array_stride: 3 * std::mem::size_of::<f32>() as u64,
    step_mode: wgpu::VertexStepMode::Instance,
    attributes: &vertex_attr_array![0 => Float32x3],
  };

  impl Simulation for DefaultSim {
    fn step(&mut self, _dt: f32) {}

    fn run_passes(
      &mut self,
      mut encoder: wgpu::CommandEncoder,
      global_bind_group: &wgpu::BindGroup,
      view: &wgpu::TextureView,
    ) -> wgpu::CommandBuffer {
      self.compute(&mut encoder, global_bind_group);

      {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
          label: None,
          color_attachments: &[Some(RenderPassColorAttachment {
            view,
            resolve_target: None,
            ops: wgpu::Operations {
              load: wgpu::LoadOp::Load,
              store: wgpu::StoreOp::Store,
            },
          })],
          depth_stencil_attachment: None,
          timestamp_writes: None,
          occlusion_query_set: None,
        });
        pass.set_pipeline(self.pipeline.as_ref().unwrap());
        self.render_into_pass(global_bind_group, &mut pass);
      }
      encoder.finish()
    }

    fn init_pipelines(
      &mut self,
      device: &wgpu::Device,
      format: wgpu::TextureFormat,
      global_layout: &wgpu::BindGroupLayout,
    ) {
      // CELL BINDING GROUP (for DRAWING & COMPUTE)
      let grid_bg_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("Cell grid binding layout"),
        entries: &[wgpu::BindGroupLayoutEntry {
          binding: 0,
          visibility: wgpu::ShaderStages::all(),
          ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Storage { read_only: false },
            has_dynamic_offset: false,
            min_binding_size: None,
          },
          count: None,
        }],
      });
      
      // PARAMETER BUFFER
      let params_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("SimParams bg layout"),
        entries: &[wgpu::BindGroupLayoutEntry {
          binding: 0,
          visibility: ShaderStages::COMPUTE,
          ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Storage { read_only: true },
            has_dynamic_offset: false,
            min_binding_size: None,
          },
          count: None,
        }],
      });
      let params_buf = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Simulation params"),
        size: std::mem::size_of::<SimulationParams>() as u64,
        usage: BufferUsages::COPY_DST | BufferUsages::STORAGE,
        mapped_at_creation: false,
      });
      let params_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("SimParams BG itself"),
        layout: &params_layout,
        entries: &[wgpu::BindGroupEntry {
          binding: 0,
          resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
            buffer: &params_buf,
            offset: 0,
            size: None,
          }),
        }],
      });
      // COMPUTE PIPELINES
      let compute_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("DefSim COMPUTE pipeline layout"),
        bind_group_layouts: &[
          global_layout,
          &grid_bg_layout,
          &self.positions.cur_layout(),
          self.cells.cur_layout(),
          &params_layout
        ],
        push_constant_ranges: &[],
      });

      let compute_module =
        device.create_shader_module(wgpu::include_wgsl!("shaders/2d-compute.wgsl"));

      let mut compute_desc = wgpu::ComputePipelineDescriptor {
        label: Some("DefSim COMPUTE pipeline"),
        layout: Some(&compute_layout),
        module: &compute_module,
        entry_point: Some("apply_velocities"),
        compilation_options: Default::default(),
        cache: None,
      };
      let compute_pipeline = device.create_compute_pipeline(&compute_desc);
      compute_desc.entry_point = Some("mass_conservation");
      let mass_consv_pipeline = device.create_compute_pipeline(&compute_desc);

      // PARTICLE DRAWING (geometry)

      // DRAWING layout
      let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: None,
        bind_group_layouts: &[global_layout, &grid_bg_layout, self.cells.cur_layout()],
        push_constant_ranges: &[],
      });

      let module = device.create_shader_module(wgpu::include_wgsl!("shaders/2d-basic.wgsl"));

      let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
        label: Some("Default simulation"),
        layout: Some(&layout),
        vertex: wgpu::VertexState {
          module: &module,
          entry_point: Some("vs_main"),
          compilation_options: Default::default(),
          buffers: &[DEFSIM_BUFFER_LAYOUT],
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
        fragment: Some(wgpu::FragmentState {
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

      let squarebuffer = device.create_buffer_init(&BufferInitDescriptor {
        label: Some("Unit square buffer"),
        contents: SQUARE.as_bytes_buffer(),
        usage: BufferUsages::VERTEX,
      });
      let celldensity = device.create_render_pipeline(&RenderPipelineDescriptor {
        label: Some("Cell density pipeline"),
        layout: Some(&layout),
        vertex: wgpu::VertexState {
          module: &module,
          entry_point: Some("vs_density"),
          compilation_options: Default::default(),
          buffers: &[wgpu::VertexBufferLayout {
            array_stride: (size_of::<f32>() * 2) as u64,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &vertex_attr_array![0 => Float32x2],
          }],
        },
        primitive: PrimitiveState {
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
        fragment: Some(FragmentState {
          module: &module,
          entry_point: Some("fs_density"),
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

      let grid_buf = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Cell dimension buffer"),
        // u32<WIDTH> | u32<HEIGHT> | f32<SIDE> | f32<VMIN> | f32<VMAX>
        size: 2 * std::mem::size_of::<u32>() as u64 +  3 * std::mem::size_of::<f32>() as u64,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::STORAGE,
        mapped_at_creation: false,
      });
      let grid_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("Cell bind group"),
        layout: &grid_bg_layout,
        entries: &[wgpu::BindGroupEntry {
          binding: 0,
          resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
            buffer: &grid_buf,
            offset: 0,
            size: None,
          }),
        }],
      });

      self.pipeline = Some(pipeline);
      self.move_pipeline = Some(compute_pipeline);
      self.mass_consv_pipeline = Some(mass_consv_pipeline);

      self.grid_buf = Some(grid_buf);
      self.grid_bg = Some(grid_bg);

      self.params_bg = Some(params_bg);
      self.params_buf = Some(params_buf);

      self.density_pipeline = Some(celldensity);
      self.square_buffer = Some(squarebuffer);
    }

    fn write_buffers(&self, queue: &wgpu::Queue) {
      let celldims = (self.x_cells as u32, self.y_cells as u32);
      let a: Vec<_> = [celldims.0, celldims.1]
        .into_iter()
        .map(|x| x.to_ne_bytes())
        .flatten()
        .chain(self.opts.size.to_ne_bytes().into_iter())
        .chain(self.opts.vmin.to_ne_bytes())
        .chain(self.opts.vmax.to_ne_bytes())
        .collect();

      queue.write_buffer(self.grid_buf.as_ref().unwrap(), 0, &a);
      queue.write_buffer(
        self.params_buf.as_ref().unwrap(),
        0,
        self.params.as_bytes_buffer(),
      );
    }

    fn on_surface_resized(&mut self, size: egui::Vec2, device: &wgpu::Device) {
      self.width = size.x;
      self.height = size.y;
      self.regenerate_positions(device);
      self.regenerate_grid(device, self.opts);
    }
  }
}
