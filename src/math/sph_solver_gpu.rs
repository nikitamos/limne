use wgpu::{
  BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor,
  BindGroupLayoutEntry, Buffer, BufferDescriptor, ComputePassDescriptor, ComputePipeline,
  ComputePipelineDescriptor, PipelineLayoutDescriptor, ShaderStages,
};

use crate::render::{
  render_target::{ExternalResources, RenderTarget},
  swapchain::SwapBuffers,
};

pub use super::sph_solver::Particle;

pub struct SphSolverGpu {
  density_pressure: ComputePipeline,
  pressure_forces: ComputePipeline,
  integrate_forces: ComputePipeline,
  pressure_buf: Buffer,
  pressure_bg: BindGroup,
  count: u32,
}

pub struct SphSolverGpuRenderResources<'a> {
  pub pos: &'a SwapBuffers<Vec<Particle>>,
  pub global_bg: &'a wgpu::BindGroup,
  pub params_buf: &'a wgpu::Buffer,
}

impl ExternalResources<'_> for SphSolverGpuRenderResources<'_> {}

impl<'a> RenderTarget<'a> for SphSolverGpu {
  type RenderResources = SphSolverGpuRenderResources<'a>;
  type InitResources = (usize, &'a BindGroupLayout);
  type UpdateResources = Self::RenderResources;

  fn update(
    &mut self,
    _device: &wgpu::Device,
    _queue: &wgpu::Queue,
    resources: &'a Self::UpdateResources,
    encoder: &mut wgpu::CommandEncoder,
  ) {
    let mut pass = encoder.begin_compute_pass(&ComputePassDescriptor {
      label: Some("SPH Solver compute pass"),
      timestamp_writes: None,
    });
    self.setup_groups_for_compute(&self.density_pressure, resources, &mut pass);
    pass.dispatch_workgroups(self.count, 1, 1);

    self.setup_groups_for_compute(&self.pressure_forces, resources, &mut pass);
    pass.dispatch_workgroups(self.count, 1, 1);

    self.setup_groups_for_compute(&self.integrate_forces, resources, &mut pass);
    pass.dispatch_workgroups(self.count, 1, 1);
  }

  fn render_into_pass(&self, _pass: &mut wgpu::RenderPass, _resources: &'a Self::RenderResources) {
    // nop
  }
}

impl SphSolverGpu {
  fn setup_groups_for_compute<'a>(
    &self,
    pipeline: &ComputePipeline,
    resources: &SphSolverGpuRenderResources<'a>,
    pass: &mut wgpu::ComputePass<'_>,
  ) {
    pass.set_pipeline(pipeline);
    pass.set_bind_group(0, resources.pos.cur_group(), &[]);
    pass.set_bind_group(1, &self.pressure_bg, &[]);
    pass.set_bind_group(2, resources.global_bg, &[]);
  }
  pub fn new<'a>(
    device: &wgpu::Device,
    resources: &'a <SphSolverGpu as RenderTarget>::RenderResources,
    init_res: <SphSolverGpu as RenderTarget>::InitResources,
  ) -> Self {
    let pressure_buf = device.create_buffer(&BufferDescriptor {
      label: None,
      size: (std::mem::size_of::<f32>() * init_res.0) as u64,
      usage: wgpu::BufferUsages::STORAGE,
      mapped_at_creation: false,
    });
    let module = device.create_shader_module(wgpu::include_wgsl!("sph-solver.wgsl"));
    let bg_layout_1 = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
      label: None,
      entries: &[
        BindGroupLayoutEntry {
          binding: 0,
          visibility: ShaderStages::COMPUTE,
          ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Storage { read_only: false },
            has_dynamic_offset: false,
            min_binding_size: None,
          },
          count: None,
        },
        BindGroupLayoutEntry {
          binding: 1,
          visibility: ShaderStages::COMPUTE,
          ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Storage { read_only: true },
            has_dynamic_offset: false,
            min_binding_size: None,
          },
          count: None,
        },
      ],
    });
    let pressure_bg = device.create_bind_group(&BindGroupDescriptor {
      label: None,
      layout: &bg_layout_1,
      entries: &[
        BindGroupEntry {
          binding: 0,
          resource: pressure_buf.as_entire_binding(),
        },
        BindGroupEntry {
          binding: 1,
          resource: resources.params_buf.as_entire_binding(),
        },
      ],
    });

    let layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
      label: None,
      bind_group_layouts: &[resources.pos.cur_layout(), &bg_layout_1, init_res.1],
      push_constant_ranges: &[],
    });
    let density_pressure = device.create_compute_pipeline(&ComputePipelineDescriptor {
      label: Some("Density&Pressure"),
      layout: Some(&layout),
      module: &module,
      entry_point: Some("density_pressure"),
      compilation_options: Default::default(),
      cache: None,
    });
    let pressure_forces = device.create_compute_pipeline(&ComputePipelineDescriptor {
      label: Some("Pressure Forces"),
      layout: Some(&layout),
      module: &module,
      entry_point: Some("pressure_forces"),
      compilation_options: Default::default(),
      cache: None,
    });
    let integrate_forces = device.create_compute_pipeline(&ComputePipelineDescriptor {
      label: Some("Integrate Forces"),
      layout: Some(&layout),
      module: &module,
      entry_point: Some("integrate_forces"),
      compilation_options: Default::default(),
      cache: None,
    });
    Self {
      density_pressure,
      pressure_forces,
      integrate_forces,
      pressure_buf,
      pressure_bg,
      count: init_res.0 as u32,
    }
  }
}
