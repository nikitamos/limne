use core::{f32, slice};

use cgmath::{Point3, Vector3};
use rayon::prelude::*;
use wgpu::VertexBufferLayout;

use crate::math::sph_solver::Particle;
use crate::math::sph_solver_gpu::{SphSolverGpu, SphSolverGpuRenderResources};
use crate::render::swapchain::{SwapBuffers, SwapBuffersDescriptor};
use crate::render::AsBuffer;

impl AsBuffer for Vec<Particle> {
  fn as_bytes_buffer(&self) -> &[u8] {
    unsafe {
      slice::from_raw_parts(
        self.as_ptr().cast(),
        self.len() * std::mem::size_of::<Particle>(),
      )
    }
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
  pub h: f32,
  pub rho0: f32,
  pub paused: bool,
  pub draw_particles: bool,
  pub regen_particles: bool,
  pub move_particles: bool,
}

impl Default for SimulationParams {
  fn default() -> Self {
    Self {
      k: 0.3,
      m0: 0.01,
      viscosity: 0.0,
      h: 10.0,
      rho0: 3.0,
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
use wgpu::{
  vertex_attr_array, BufferUsages, DepthStencilState, MultisampleState, RenderPipelineDescriptor,
  ShaderStages,
};

use crate::render::render_target::{ExternalResources, RenderTarget};

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
  pub dt: f32,
}

pub struct SimInit<'a> {
  pub count: usize,
  pub size: egui::Vec2,
  pub depth_state: &'a wgpu::DepthStencilState,
}

impl<'a> ExternalResources<'a> for SimResources<'a> {}

pub struct SphSimulation {
  // positions: SwapBuffers<ParticleVector<f32>>,
  pos_buf: Option<SwapBuffers<Vec<Particle>>>,
  pipeline: Option<wgpu::RenderPipeline>,
  params_buf: Option<wgpu::Buffer>,
  params_bg: Option<wgpu::BindGroup>,
  height: f32,
  width: f32,
  opts: SimulationRegenOptions,
  count: usize,
  solver: Option<SphSolverGpu>,
}

const DEFSIM_BUFFER_LAYOUT: VertexBufferLayout = VertexBufferLayout {
  array_stride: std::mem::size_of::<Particle>() as u64,
  step_mode: wgpu::VertexStepMode::Instance,
  attributes: &vertex_attr_array![0 => Float32x3],
};

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
      resources.regen_options.unwrap(),
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
    self.pos_buf.as_mut().unwrap().swap(encoder);
    if resources.params.regen_particles {
      self.regenerate_positions(device);
    }
    if !resources.params.paused {
      self.solver.as_mut().unwrap().update(device, queue, &SphSolverGpuRenderResources {
        pos: self.pos_buf.as_mut().unwrap(),
        global_bg: resources.global_group,
        params_buf: self.params_buf.as_mut().unwrap()
    }, encoder);
    }
    self.write_buffers(queue, resources.params);
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
    self.init_pipelines(
      device,
      format,
      &resources.global_layout,
      &resources.depth_stencil,
    );
  }

  fn render_into_pass(&self, pass: &mut wgpu::RenderPass, resources: &'a Self::RenderResources) {
    if resources.params.draw_particles {
      pass.set_pipeline(self.pipeline.as_ref().unwrap());
      pass.set_vertex_buffer(0, self.pos_buf.as_ref().unwrap().cur_buf().slice(..));
      self.setup_groups_for_render(resources.global_group, pass);
      pass.draw(0..3, 0..(self.count as u32));
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
    opts: SimulationRegenOptions,
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
      pipeline: None,
      params_buf: None,
      params_bg: None,
      height,
      width,
      opts,
      count,
      solver: None,
    };
    out.init_pipelines(device, format, global_layout, depth);
    out.regenerate_positions(device);
    out
  }

  fn setup_groups_for_render(
    &self,
    global_bind_group: &wgpu::BindGroup,
    pass: &mut wgpu::RenderPass<'_>,
  ) {
    pass.set_bind_group(0, global_bind_group, &[]);
  }

  fn regenerate_positions(&mut self, device: &wgpu::Device) {
    let r_distr = rand::distr::Uniform::new(0., 400.0).unwrap();

    let v_distr = rand::distr::Uniform::new(0., 30.).unwrap();
    let theta = rand::distr::Uniform::new(0., f32::consts::PI).unwrap();
    let phi = rand::distr::Uniform::new(0., f32::consts::TAU).unwrap();

    let mut parts = vec![Particle::default(); self.count];

    parts.par_iter_mut().for_each(|p| {
      let mut rng = rand::rng();
      let mtheta = rng.sample(theta);
      let mphi = rng.sample(phi);
      let r = rng.sample(r_distr);
      p.pos = Point3 {
        x: r * mtheta.sin() * mphi.cos(),
        y: r * mtheta.sin() * mphi.sin(),
        z: r * mtheta.cos(),
      };

      let v = rng.sample(v_distr);
      let theta = rng.sample(theta);
      let phi = rng.sample(phi);
      p.velocity = Vector3 {
        x: v * theta.sin() * phi.cos(),
        y: v * theta.sin() * phi.sin(),
        z: v * theta.cos(),
      }
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

    let solver = SphSolverGpu::new(
      device,
      &SphSolverGpuRenderResources {
        pos: &pos_buf,
        global_bg: &params_bg,
        params_buf: &params_buf,
      },
      (self.count, &global_layout),
    );

    self.pipeline = Some(pipeline);
    self.pos_buf = Some(pos_buf);
    self.params_bg = Some(params_bg);
    self.params_buf = Some(params_buf);
    self.solver = Some(solver);
  }

  fn write_buffers(&self, queue: &wgpu::Queue, params: &SimulationParams) {
    queue.write_buffer(
      self.params_buf.as_ref().unwrap(),
      0,
      params.as_bytes_buffer(),
    );
    // queue.write_buffer(
    //   self.pos_buf.as_ref().unwrap(),
    //   0,
    //   self.solver.particles().as_bytes_buffer(),
    // );
  }
}
