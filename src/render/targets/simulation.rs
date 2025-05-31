use core::{f32, slice};

use cgmath::{Point3, Vector3, Zero};
use rayon::prelude::*;

use crate::render::swapchain::{SwapBuffers, SwapBuffersDescriptor};
use crate::render::AsBuffer;
use crate::solvers::sph_solver_gpu::Particle;
use crate::solvers::sph_solver_gpu::{SphSolverGpu, SphSolverGpuRenderResources};

#[repr(C)]
#[derive(Clone, Copy)]
pub struct SimulationParams {
  pub k: f32,
  pub m0: f32,
  pub viscosity: f32,
  pub h: f32,
  pub rho0: f32,
  pub e: f32,
  pub w: f32,
  pub paused: bool,
  pub draw_particles: bool,
  pub regen_particles: bool,
  pub move_particles: bool,
}

impl Default for SimulationParams {
  fn default() -> Self {
    Self {
      k: 40.0,
      m0: 30.0,
      viscosity: 0.0,
      h: 2.0,
      rho0: 5.0,
      e: 0.8,
      w: 20.0,
      paused: false,
      draw_particles: true,
      regen_particles: false,
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

use rand::Rng;
use wgpu::{BufferUsages, DepthStencilState, ShaderStages};

use crate::render::render_target::{ExternalResources, RenderTarget};

use super::fluid_renderer::{FluidRenderInit, FluidRenderer, FluidRendererResources};
use crate::render::blur::{Blur, GaussianBlur};

pub struct SimResources<'a> {
  pub params: &'a SimulationParams,
  pub global_group: &'a wgpu::BindGroup,
  pub global_layout: &'a wgpu::BindGroupLayout,
  pub depth_stencil: &'a wgpu::DepthStencilState,
}

pub struct SimUpdateResources<'a> {
  pub params: &'a SimulationParams,
  pub global_group: &'a wgpu::BindGroup,
  pub global_layout: &'a wgpu::BindGroupLayout,
  pub depth_stencil: &'a wgpu::DepthStencilState,
  pub dt: f32,
}

pub struct SimInit<'a> {
  pub count: usize,
  pub size: egui::Vec2,
  pub depth_state: &'a wgpu::DepthStencilState,
}

impl<'a> ExternalResources<'a> for SimResources<'a> {}

pub struct SphSimulation {
  pos_buf: Option<SwapBuffers<Vec<Particle>>>,
  fluid_renderer: Option<FluidRenderer>,
  params_buf: Option<wgpu::Buffer>,
  params_bg: Option<wgpu::BindGroup>,
  height: f32,
  width: f32,
  count: usize,
  solver: Option<SphSolverGpu>,
  smoother: Box<dyn Blur + Sync + Send>,
}

impl<'a> RenderTarget<'a> for SphSimulation {
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
      init_res.depth_state,
    )
  }

  fn update(
    &mut self,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    resources: &'a Self::UpdateResources,
    encoder: &mut wgpu::CommandEncoder,
  ) {
    if !resources.params.paused {
      self.pos_buf.as_mut().unwrap().swap(encoder);
      self.write_buffers(queue, resources.params);
    }

    if resources.params.regen_particles {
      self.regenerate_positions(device);
    }
    if !resources.params.paused {
      self.solver.as_mut().unwrap().update(
        device,
        queue,
        &SphSolverGpuRenderResources {
          pos: self.pos_buf.as_mut().unwrap(),
          global_bg: resources.global_group,
          params_buf: self.params_buf.as_mut().unwrap(),
        },
        encoder,
      );
    }
    if resources.params.draw_particles {
      self.fluid_renderer.as_mut().unwrap().update(
        device,
        queue,
        &FluidRendererResources {
          global_bg: resources.global_group,
          params_bg: self.params_bg.as_ref().unwrap(),
          pos_buf: self.pos_buf.as_ref().unwrap().cur_buf(),
          count: self.count as u32,
        },
        encoder,
      );
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
    self.init_pipelines(
      device,
      format,
      resources.global_layout,
      resources.depth_stencil,
    );
    // FIXME: Is it necessary to do? isn't it enough to call `init_pipelines`?
    self.fluid_renderer.as_mut().unwrap().resized(
      device,
      new_size,
      &FluidRendererResources {
        global_bg: resources.global_group,
        params_bg: self.params_bg.as_ref().unwrap(),
        pos_buf: self.pos_buf.as_ref().unwrap().cur_buf(),
        count: self.count as u32,
      },
      format,
    );
  }

  fn render_into_pass(&self, pass: &mut wgpu::RenderPass, resources: &'a Self::RenderResources) {
    if resources.params.draw_particles {
      self.fluid_renderer.as_ref().unwrap().render_into_pass(
        pass,
        &FluidRendererResources {
          global_bg: resources.global_group,
          params_bg: self.params_bg.as_ref().unwrap(),
          pos_buf: self.pos_buf.as_ref().unwrap().cur_buf(),
          count: self.count as u32,
        },
      );
    }
  }
}

impl SphSimulation {
  fn create_fully_initialized(
    count: usize,
    device: &wgpu::Device,
    size: egui::Vec2,
    format: wgpu::TextureFormat,
    global_layout: &wgpu::BindGroupLayout,
    depth: &DepthStencilState,
  ) -> Self {
    let mut particles: Vec<Particle> = vec![Default::default(); count];
    let mut rng = rand::rng();
    let width = size.x;
    let height = size.y;

    for p in particles.iter_mut() {
      p.pos.x = rng.sample(rand::distr::Uniform::new(0.0, width).unwrap());
      p.pos.y = rng.sample(rand::distr::Uniform::new(0.0, height).unwrap());
    }

    let mut out = Self {
      // Initialized in `init_pipelines`
      pos_buf: None,
      fluid_renderer: None,
      params_buf: None,
      params_bg: None,
      height,
      width,
      count,
      solver: None,
      smoother: Box::new(GaussianBlur::default()),
    };
    out.init_pipelines(device, format, global_layout, depth);
    out.regenerate_positions(device);
    out
  }

  fn regenerate_positions(&mut self, device: &wgpu::Device) {
    let w_distr = rand::distr::Uniform::new(-10.0, 20.0).unwrap();

    let mut parts = vec![Particle::default(); self.count];

    parts.par_iter_mut().for_each(|p| {
      let mut rng = rand::rng();
      p.pos = Point3 {
        x: rng.sample(w_distr),
        y: rng.sample(w_distr),
        z: rng.sample(w_distr),
      };

      p.velocity = Vector3::zero();
    });
    self.pos_buf.as_mut().unwrap().reset(parts, device);
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
        visibility: ShaderStages::all(),
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
      usage: BufferUsages::COPY_DST | BufferUsages::STORAGE | BufferUsages::UNIFORM,
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

    let pos_buf = SwapBuffers::init_with(
      vec![Default::default(); self.count],
      device,
      SwapBuffersDescriptor {
        usage: BufferUsages::VERTEX
          | BufferUsages::COPY_DST
          | BufferUsages::COPY_SRC
          | BufferUsages::STORAGE,
        visibility: ShaderStages::all(),
        ty: wgpu::BufferBindingType::Storage { read_only: false },
        has_dynamic_offset: false,
      },
    );

    let solver = SphSolverGpu::new(device, (self.count, global_layout, &params_buf, &pos_buf));
    let fluid_renderer = FluidRenderer::new(
      device,
      &format,
      FluidRenderInit {
        size: egui::Vec2::new(self.width, self.height),
        global_layout,
        params_layout: &params_layout,
        depth_stencil_state: depth_stencil.clone(),
        smoother_matrix: self.smoother.full_kernel(),
      },
    );

    self.fluid_renderer = Some(fluid_renderer);
    self.pos_buf = Some(pos_buf);
    self.params_bg = Some(params_bg);
    self.params_buf = Some(params_buf);
    self.solver = Some(solver);
    self.regenerate_positions(device);
  }

  fn write_buffers(&self, queue: &wgpu::Queue, params: &SimulationParams) {
    queue.write_buffer(
      self.params_buf.as_ref().unwrap(),
      0,
      params.as_bytes_buffer(),
    );
  }
}
