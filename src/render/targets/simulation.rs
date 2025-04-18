use core::slice;
use std::ops::{Deref, DerefMut};

use crate::math::vector::NumVector3D;
use wgpu::VertexBufferLayout;

use crate::render::AsBuffer;

#[rustfmt::skip]
const SQUARE: [f32; 12] = [
  0., 0.,
  0., 1.,
  1., 1.,
  1., 0.,
  0., 0.,
  1., 1.,
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

#[derive(Clone, Copy)]
pub struct SimulationRegenOptions {
  pub size: f32,
  pub vmin: f32,
  pub vmax: f32,
}

impl Default for SimulationRegenOptions {
  fn default() -> Self {
    Self {
      size: 5.0,
      vmin: 2.0,
      vmax: 7.5,
    }
  }
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct SimulationParams {
  pub k: f32,
  pub m0: f32,
  pub viscosity: f32,
  pub paused: bool,
  pub draw_particles: bool,
  pub draw_density_field: bool,
  pub move_particles: bool,
}

impl Default for SimulationParams {
  fn default() -> Self {
    Self {
      k: 0.3,
      m0: 0.01,
      viscosity: 0.0,
      paused: false,
      draw_particles: false,
      draw_density_field: true,
      move_particles: true,
    }
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

fn make_vec_buf<T>(v: &Vec<T>) -> &[u8] {
  unsafe { slice::from_raw_parts(v[..].as_ptr().cast(), v.len() * std::mem::size_of::<T>()) }
}

impl AsBuffer for Vec<DefaultCell> {
  fn as_bytes_buffer(&self) -> &[u8] {
    make_vec_buf(self)
  }
}

use core::f32;
use rand::Rng;
use wgpu::{
  vertex_attr_array, BufferUsages, DepthStencilState, MultisampleState, RenderPipelineDescriptor,
  ShaderStages,
};

use crate::render::{
  render_target::{ExternalResources, RenderTarget},
  swapchain::{SwapBuffers, SwapBuffersDescriptor},
};

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

pub struct SimResources<'a> {
  pub params: &'a SimulationParams,
  pub global_group: &'a wgpu::BindGroup,
  pub global_layout: &'a wgpu::BindGroupLayout,
  pub depth_stencil: &'a wgpu::DepthStencilState,
  pub regen_options: Option<SimulationRegenOptions>,
}

pub struct SimUpdateResources<'a> {
  pub params: &'a SimulationParams,
  pub global_group: &'a wgpu::BindGroup,
  pub global_layout: &'a wgpu::BindGroupLayout,
  pub depth_stencil: &'a wgpu::DepthStencilState,
}

pub struct SimInit<'a> {
  pub count: usize,
  pub size: egui::Vec2,
  pub depth_state: &'a wgpu::DepthStencilState,
}

impl<'a> ExternalResources<'a> for SimResources<'a> {}

pub struct DefaultSim {
  positions: SwapBuffers<ParticleVector<f32>>,
  pipeline: Option<wgpu::RenderPipeline>,
  params_buf: Option<wgpu::Buffer>,
  params_bg: Option<wgpu::BindGroup>,
  height: f32,
  width: f32,
  opts: SimulationRegenOptions,
}

const DEFSIM_BUFFER_LAYOUT: VertexBufferLayout = VertexBufferLayout {
  array_stride: 3 * std::mem::size_of::<f32>() as u64,
  step_mode: wgpu::VertexStepMode::Instance,
  attributes: &vertex_attr_array![0 => Float32x3],
};

impl<'a> RenderTarget<'a> for DefaultSim {
  type RenderResources = SimResources<'a>;
  type InitResources = SimInit<'a>;
  type UpdateResources = SimUpdateResources<'a>;

  fn init(
    device: &wgpu::Device,
    _queue: &wgpu::Queue,
    resources: &'a Self::RenderResources,
    format: &wgpu::TextureFormat,
    init_res: Self::InitResources,
  ) -> Self {
    Self::create_fully_initialized(
      init_res.count,
      device,
      init_res.size,
      *format,
      resources.global_layout,
      resources.regen_options.unwrap(),
      init_res.depth_state,
    )
  }

  fn update(
    &mut self,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    resources: &'a Self::UpdateResources,
    _encoder: &mut wgpu::CommandEncoder,
  ) {
    self.write_buffers(queue, resources.params);
    if !resources.params.paused {
      self.regenerate_positions(device);
    }
  }

  fn resized(
    &mut self,
    device: &wgpu::Device,
    new_size: egui::Vec2,
    resources: &'a Self::UpdateResources,
    format: wgpu::TextureFormat,
  ) {
    self.width = new_size.x;
    self.height = new_size.y;
    self.regenerate_positions(device);
    self.reinit_pipelines(
      device,
      format,
      &resources.global_layout,
      &resources.depth_stencil,
    );
  }

  fn render_into_pass(&self, pass: &mut wgpu::RenderPass, resources: &'a Self::RenderResources) {
    if resources.params.draw_particles {
      pass.set_pipeline(self.pipeline.as_ref().unwrap());
      pass.set_vertex_buffer(0, self.positions.cur_buf().slice(..));
      self.setup_groups_for_render(resources.global_group, pass);
      pass.draw(0..3, 0..(self.positions.cur().len() as u32));
    }
  }
}

impl DefaultSim {
  pub fn create_fully_initialized(
    count: usize,
    device: &wgpu::Device,
    size: egui::Vec2,
    format: wgpu::TextureFormat,
    global_layout: &wgpu::BindGroupLayout,
    opts: SimulationRegenOptions,
    depth: &DepthStencilState,
  ) -> Self {
    let mut out = Self::new(count, device, size, opts);
    out.init_pipelines(device, format, global_layout, depth);
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

    Self {
      positions: SwapBuffers::init_with(
        positions.into(),
        device,
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
      params_buf: None,
      params_bg: None,
      height,
      width,
      opts,
    }
  }

  fn setup_groups_for_render(
    &self,
    global_bind_group: &wgpu::BindGroup,
    pass: &mut wgpu::RenderPass<'_>,
  ) {
    pass.set_bind_group(0, global_bind_group, &[]);
  }

  fn regenerate_positions(&mut self, device: &wgpu::Device) {
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

  fn init_pipelines(
    &mut self,
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    global_layout: &wgpu::BindGroupLayout,
    depth_stencil: &DepthStencilState,
  ) {
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

    // PARTICLE DRAWING (geometry)

    // DRAWING layout
    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
      label: None,
      bind_group_layouts: &[global_layout],
      push_constant_ranges: &[],
    });

    let module =
      device.create_shader_module(wgpu::include_wgsl!("shaders/simulation-particles.wgsl"));

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
        cull_mode: None, //Some(wgpu::Face::Back),
        unclipped_depth: false,
        polygon_mode: wgpu::PolygonMode::Fill,
        conservative: false,
      },
      depth_stencil: Some(depth_stencil.clone()),
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

    self.pipeline = Some(pipeline);

    self.params_bg = Some(params_bg);
    self.params_buf = Some(params_buf);
  }

  fn write_buffers(&self, queue: &wgpu::Queue, params: &SimulationParams) {
    queue.write_buffer(
      self.params_buf.as_ref().unwrap(),
      0,
      params.as_bytes_buffer(),
    );
  }

  #[deprecated]
  fn reinit_pipelines(
    &mut self,
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    global_layout: &wgpu::BindGroupLayout,
    depth_stencil: &DepthStencilState,
  ) {
    self.init_pipelines(device, format, global_layout, depth_stencil);
  }
}
