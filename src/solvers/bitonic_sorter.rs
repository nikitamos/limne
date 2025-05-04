use wgpu::{
  ComputePassDescriptor, ComputePipelineDescriptor, PipelineLayoutDescriptor, PushConstantRange,
  ShaderStages,
};

/// Count of workgroups in a local pass that sorts subarrays of length `2*LOCAL_PASS_SIZE`
/// and performs disperse using local memory optimizations.
pub const LOCAL_PASS_SIZE: u32 = 512;
pub const LOCAL_ARRAY_SIZE: u32 = 2 * LOCAL_PASS_SIZE;
const LOG_LOCAL_ARRAY_SIZE: u32 = LOCAL_ARRAY_SIZE.trailing_zeros();
pub const GLOBAL_PASS_SIZE: u32 = 64;

#[cfg(test)]
mod test {
  use core::slice;

  use rand::distr::Distribution;
  use wgpu::{
    BufferUsages, ComputePassDescriptor, Features, InstanceDescriptor, Limits,
    RequestAdapterOptions, ShaderStages,
  };

  use crate::{
    render::swapchain::{SwapBuffers, SwapBuffersDescriptor},
    solvers::{bitonic_sorter::ParticleBitonicSorter, sph_solver_gpu::Particle},
  };

  async fn setup_wgpu() -> Result<(wgpu::Device, wgpu::Queue), ()> {
    let instance = wgpu::Instance::new(&InstanceDescriptor {
      backends: wgpu::Backends::PRIMARY,
      ..Default::default()
    });
    let adapter = instance
      .request_adapter(&RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::HighPerformance,
        force_fallback_adapter: false,
        compatible_surface: None,
      })
      .await
      .expect("Unable to request an adapter");
    adapter
      .request_device(
        &wgpu::DeviceDescriptor {
          required_limits: Limits {
            max_compute_invocations_per_workgroup: 512,
            max_compute_workgroup_size_x: 512,
            max_compute_workgroup_storage_size: size_of::<Particle>() as u32 * 1024,
            max_push_constant_size: 8,
            ..Default::default()
          },
          required_features: Features::PUSH_CONSTANTS,
          ..Default::default()
        },
        None,
      )
      .await
      .map_err(|_| ())
  }

  #[tokio::test]
  async fn gpu_bitonic_sort_1024() -> Result<(), ()> {
    let (device, ref mut queue) = setup_wgpu().await?;
    let array = particle_array(1024, -20.0, 500.0).await;
    let (mut buf, obuf) = particle_gpu(array, &device).await;

    buf.write(queue);
    let sorter = ParticleBitonicSorter::new(&device, buf.cur_layout());
    let mut encoder =
      device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
    sorter.sort(&mut encoder, buf.cur_group(), 1024);
    let cmd = encoder.finish();
    let mut e = device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
    e.copy_buffer_to_buffer(buf.cur_buf(), 0, &obuf, 0, 48 * 1024);
    queue.submit([cmd]);
    queue.submit([e.finish()]);

    {
      let obuf = obuf.clone();
      obuf
        .clone()
        .slice(..)
        .map_async(wgpu::MapMode::Read, move |a| unsafe {
          a.unwrap();
          let v = slice::from_raw_parts::<Particle>(
            obuf.slice(..).get_mapped_range().as_ptr().cast(),
            1024,
          );
          if let Err(fail) = is_sorted(v) {
            panic!("Array is not sorted. First element out of order has index {fail}");
          }

          obuf.unmap();
        });
    }
    tokio::spawn(async move { device.poll(wgpu::MaintainBase::Wait) });

    Ok(())
  }

  #[tokio::test]
  async fn gpu_bitonic_sort_local_x2() -> Result<(), ()> {
    let (device, ref mut queue) = setup_wgpu().await?;
    let array = particle_array(2048, 0., 2048.).await;
    let (mut buf, obuf) = particle_gpu(array, &device).await;

    buf.write(queue);
    let sorter = ParticleBitonicSorter::new(&device, buf.cur_layout());
    let mut encoder =
      device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
    let mut pass = encoder.begin_compute_pass(&ComputePassDescriptor {
      label: None,
      timestamp_writes: None,
    });
    sorter.sort_local(&mut pass, buf.cur_group(), 2);
    std::mem::drop(pass);

    let cmd = encoder.finish();
    let mut e = device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
    e.copy_buffer_to_buffer(buf.cur_buf(), 0, &obuf, 0, 48 * 2048);
    queue.submit([cmd]);
    queue.submit([e.finish()]);

    {
      let obuf = obuf.clone();
      obuf
        .clone()
        .slice(..)
        .map_async(wgpu::MapMode::Read, move |a| unsafe {
          a.unwrap();
          let v = slice::from_raw_parts::<Particle>(
            obuf.slice(..).get_mapped_range().as_ptr().cast(),
            2048,
          );
          if let Err(fail) = is_sorted(&v[..1024]) {
            panic!("First half is not sorted. First element out of order has index {fail}");
          }
          if let Err(fail) = is_sorted(&v[1024..]) {
            panic!("Second half is not sorted. First element out of order has index {fail}");
          }

          obuf.unmap();
        });
    }
    tokio::spawn(async move { device.poll(wgpu::MaintainBase::Wait) });

    Ok(())
  }

  fn is_sorted(p: &[Particle]) -> Result<(), usize> {
    let mut prev = -f32::INFINITY;
    for (i, e) in p.iter().enumerate() {
      if prev > e.density {
        return Err(i);
      }
      prev = e.density;
    }
    Ok(())
  }

  #[tokio::test]
  async fn gpu_bitonic_sort_16384() -> Result<(), ()> {
    const COUNT: usize = 16384;
    let (device, ref mut queue) = setup_wgpu().await?;
    let array = particle_array(COUNT, -8192., -42.).await;
    let (mut buf, obuf) = particle_gpu(array, &device).await;

    buf.write(queue);
    let sorter = ParticleBitonicSorter::new(&device, buf.cur_layout());
    let mut encoder =
      device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
    sorter.sort(&mut encoder, buf.cur_group(), COUNT as u32);
    let cmd = encoder.finish();
    let mut e = device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
    e.copy_buffer_to_buffer(buf.cur_buf(), 0, &obuf, 0, 48 * COUNT as u64);
    queue.submit([cmd]);
    queue.submit([e.finish()]);

    {
      let obuf = obuf.clone();
      obuf
        .clone()
        .slice(..)
        .map_async(wgpu::MapMode::Read, move |a| unsafe {
          a.unwrap();
          let v = slice::from_raw_parts::<Particle>(
            obuf.slice(..).get_mapped_range().as_ptr().cast(),
            COUNT,
          );
          if let Err(fail) = is_sorted(&v) {
            panic!("The array is not sorted. First element out of order has index {fail}");
          }
          obuf.unmap();
        });
    }
    tokio::spawn(async move { device.poll(wgpu::MaintainBase::Wait) });

    Ok(())
  }

  async fn particle_gpu(
    array: Vec<Particle>,
    device: &wgpu::Device,
  ) -> (SwapBuffers<Vec<Particle>>, wgpu::Buffer) {
    let buf = SwapBuffers::init_with(
      array.clone(),
      device,
      SwapBuffersDescriptor {
        usage: BufferUsages::COPY_DST | BufferUsages::STORAGE | BufferUsages::COPY_SRC,
        visibility: ShaderStages::COMPUTE,
        ty: wgpu::BufferBindingType::Storage { read_only: false },
        has_dynamic_offset: false,
      },
    );

    let obuf = device.create_buffer(&wgpu::BufferDescriptor {
      label: Some("outbuf"),
      size: 48 * array.len() as u64,
      usage: BufferUsages::COPY_DST | BufferUsages::MAP_READ,
      mapped_at_creation: false,
    });
    (buf, obuf)
  }

  async fn particle_array(count: usize, min: f32, max: f32) -> Vec<Particle> {
    let mut rng = rand::rng();
    let d = rand::distr::Uniform::new_inclusive(min, max).unwrap();
    let mut v = vec![Particle::default(); count];
    for p in v.iter_mut() {
      p.density = d.sample(&mut rng);
    }
    v
  }
}

pub struct ParticleBitonicSorter {
  flip_local: wgpu::ComputePipeline,
  disperse_local: wgpu::ComputePipeline,
  flip_global: wgpu::ComputePipeline,
  disperse_global: wgpu::ComputePipeline,
}

impl ParticleBitonicSorter {
  pub fn new(
    device: &wgpu::Device,
    particle_layout: &wgpu::BindGroupLayout,
  ) -> ParticleBitonicSorter {
    let ref module = device.create_shader_module(wgpu::include_wgsl!("bitonic-sorter-local.wgsl"));
    let local_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
      label: Some("BitonicSorter_local"),
      bind_group_layouts: &[particle_layout],
      push_constant_ranges: &[],
    });
    let layout = Some(&local_layout);
    let flip_local = device.create_compute_pipeline(&ComputePipelineDescriptor {
      label: Some("BitonicSorter::flip_local"),
      layout,
      module,
      entry_point: Some("flip_local"),
      compilation_options: Default::default(),
      cache: None,
    });
    let disperse_local = device.create_compute_pipeline(&ComputePipelineDescriptor {
      label: Some("BitonicSorter::disperse_local"),
      layout,
      module,
      entry_point: Some("disperse_local"),
      compilation_options: Default::default(),
      cache: None,
    });

    let ref module = device.create_shader_module(wgpu::include_wgsl!("bitonic-sorter-global.wgsl"));
    let global_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
      label: Some("BitonicSorter_global"),
      bind_group_layouts: &[particle_layout],
      push_constant_ranges: &[PushConstantRange {
        stages: ShaderStages::COMPUTE,
        range: 0..8,
      }],
    });
    let layout = Some(&global_layout);
    let flip_global = device.create_compute_pipeline(&ComputePipelineDescriptor {
      label: Some("BitonicSorter::flip_global"),
      layout,
      module,
      entry_point: Some("flip_global"),
      compilation_options: Default::default(),
      cache: None,
    });
    let disperse_global = device.create_compute_pipeline(&ComputePipelineDescriptor {
      label: Some("BitonicSorter::disperse_global"),
      layout,
      module,
      entry_point: Some("disperse_global"),
      compilation_options: Default::default(),
      cache: None,
    });

    ParticleBitonicSorter {
      flip_local,
      disperse_local,
      flip_global,
      disperse_global,
    }
  }

  fn sort_local(
    &self,
    pass: &mut wgpu::ComputePass,
    particles: &wgpu::BindGroup,
    count_groups: u32,
  ) {
    pass.set_pipeline(&self.flip_local);
    pass.set_bind_group(0, particles, &[]);
    pass.dispatch_workgroups(count_groups, 1, 1);
  }

  fn disperse_local(
    &self,
    pass: &mut wgpu::ComputePass,
    particles: &wgpu::BindGroup,
    count_groups: u32,
  ) {
    pass.set_pipeline(&self.disperse_local);
    pass.set_bind_group(0, particles, &[]);
    pass.dispatch_workgroups(count_groups, 1, 1);
  }

  #[inline(always)]
  fn full_disperse_global(
    &self,
    pass: &mut wgpu::ComputePass,
    particles: &wgpu::BindGroup,
    t: u32,
    k: u32,
  ) {
    // FIXME: if sort ever fails, remove `+1`
    for q in ((LOG_LOCAL_ARRAY_SIZE + 1)..=(k - t)).rev() {
      pass.set_pipeline(&self.disperse_global);
      pass.set_bind_group(0, particles, &[]);
      pass.set_push_constants(0, &Self::pack(k, q));
      pass.dispatch_workgroups((1 << (k - 1)) / GLOBAL_PASS_SIZE, 1, 1);
    }
    self.disperse_local(pass, particles, 1 << (k - LOG_LOCAL_ARRAY_SIZE));
  }
  fn pack(a: u32, b: u32) -> [u8; 8] {
    unsafe { std::mem::transmute([a, b]) }
  }
  #[inline(always)]
  fn single_flip_global(
    &self,
    pass: &mut wgpu::ComputePass,
    particles: &wgpu::BindGroup,
    t: u32,
    k: u32,
  ) {
    debug_assert!(k >= GLOBAL_PASS_SIZE.trailing_zeros());
    pass.set_pipeline(&self.flip_global);
    pass.set_push_constants(0, &Self::pack(k, t));
    pass.set_bind_group(0, particles, &[]);
    pass.dispatch_workgroups((1 << (k - 1)) / GLOBAL_PASS_SIZE as u32, 1, 1);
  }
  pub fn sort(&self, encoder: &mut wgpu::CommandEncoder, particles: &wgpu::BindGroup, count: u32) {
    assert!(
      count >= LOCAL_ARRAY_SIZE && count.count_ones() == 1,
      "`count` must be a power of 2 greater than {LOCAL_ARRAY_SIZE}, got {count}"
    );
    let ref mut pass = encoder.begin_compute_pass(&ComputePassDescriptor {
      label: Some("BitonicSort::sort(full)"),
      timestamp_writes: None,
    });

    let k = count.trailing_zeros();
    self.sort_local(pass, particles, count >> LOG_LOCAL_ARRAY_SIZE);
    for t in (0..=(k - LOG_LOCAL_ARRAY_SIZE)).rev() {
      self.single_flip_global(pass, particles, t, k);
      self.full_disperse_global(pass, particles, t, k);
    }
  }
}
