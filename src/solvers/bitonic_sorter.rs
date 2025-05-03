use wgpu::{
  ComputePassDescriptor, ComputePipelineDescriptor, PipelineCompilationOptions,
  PipelineLayoutDescriptor, PushConstantRange, ShaderModuleDescriptor, ShaderStages,
};

/// Count of workgroups in a local pass that sorts subarrays of length `2*LOCAL_PASS_SIZE`
/// and performs disperse using local memory optimizations.
pub const LOCAL_PASS_SIZE: usize = 512;

#[cfg(test)]
mod test {
  use core::slice;
  use std::sync::Arc;

  use rand::distr::Distribution;
  use wgpu::{
    core::device::queue, BufferUsages, InstanceDescriptor, Limits, RequestAdapterOptions,
    ShaderStages,
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
            ..Default::default()
          },
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
    let mut buf = SwapBuffers::init_with(
      array.clone(),
      &device,
      SwapBuffersDescriptor {
        usage: BufferUsages::COPY_DST | BufferUsages::STORAGE | BufferUsages::COPY_SRC,
        visibility: ShaderStages::COMPUTE,
        ty: wgpu::BufferBindingType::Storage { read_only: false },
        has_dynamic_offset: false,
      },
    );

    let obuf = Arc::new(device.create_buffer(&wgpu::BufferDescriptor {
      label: Some("outbuf"),
      size: 48 * 1024 as u64,
      usage: BufferUsages::COPY_DST | BufferUsages::MAP_READ,
      mapped_at_creation: false,
    }));

    buf.write(queue);
    // queue.submit([]);
    let sorter = ParticleBitonicSorter::new(&device, queue, buf.cur_layout());
    let mut encoder =
      device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
    sorter.sort_1024(&mut encoder, buf.cur_group());
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
          let v: Vec<_> = slice::from_raw_parts::<Particle>(
            obuf.slice(..).get_mapped_range().as_ptr().cast(),
            1024,
          )
          .iter()
          .map(|p| p.density)
          .collect();
          let mut prev = -f32::INFINITY;
          for (i, e) in v.iter().enumerate() {
            if prev > *e {
              panic!("Array is not sorted: v[{i}] = {e} > {prev}\nNote: full array is {v:?}");
            }
            prev = *e;
          }

          obuf.unmap();
        });
    }
    tokio::spawn(async move { device.poll(wgpu::MaintainBase::Wait) });

    Ok(())
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

  fn cas(a: &mut [i32], l: usize, r: usize) {
    if a[l] > a[r] {
      a.swap(l, r);
    }
  }

  fn cpu_serial_bitonic_sort(a: &mut [i32]) {
    assert_eq!(a.len().count_ones(), 1, "Array length must be a power of 2");
    if a.len() == 1 {
      return;
    }
    let n = a.len();
    let k = n.trailing_zeros() as usize;
    for t in (0..=(k - 1)).rev() {
      // 2^t is count of flip blocks
      // height of a flip block?
      let flh = 1 << (k - t);
      // iterate over flip blocks
      for flb in 0..(1 << t) {
        let go = flh * flb;
        for lo in 0..flh / 2 {
          cas(a, go + lo, go + flh - lo - 1);
        }
      }
      // <BARRIER>
      // 'stages'
      for q in (1..=(k - t)).rev() {
        // 2^q is the height of disperse block in the stage
        let dbc = 1 << (k - q);
        let dbh = 1 << q;
        for i in 0..dbc {
          let go = i * dbh;
          for j in 0..dbh / 2 {
            cas(a, go + j, go + j + dbh / 2);
          }
        }
        // <BARRIER>
      }
    }
  }

  #[test]
  fn cpu_bitonic_sort_4() {
    let mut a = [2, 3, 1, 4];
    let mut b = a.clone();
    cpu_serial_bitonic_sort(&mut b);
    a.sort();
    assert_eq!(a, b);
  }

  #[test]
  fn cpu_bitonic_sort_64() {
    let mut a = array::<64>(-20, 20);
    let mut b = a.clone();
    a.sort();
    cpu_serial_bitonic_sort(&mut b);
    assert_eq!(a, b);
  }

  fn array<const N: usize>(min: i32, max: i32) -> [i32; N] {
    let mut rng = rand::rng();
    let d = rand::distr::Uniform::new_inclusive(min, max).unwrap();
    let mut a = [0; N];
    for i in a.iter_mut() {
      *i = d.sample(&mut rng);
    }
    a
  }

  #[test]
  fn cpu_bitonic_sort_1024x1024() {
    for _ in 0..1024 {
      let mut a = array::<1024>(-1000, 1000);
      let mut b = a.clone();
      a.sort();
      cpu_serial_bitonic_sort(&mut b);
      assert_eq!(a, b);
    }
  }
}

pub struct ParticleBitonicSorter {
  flip_local: wgpu::ComputePipeline,
  disperse_local: wgpu::ComputePipeline,
}

impl ParticleBitonicSorter {
  pub fn new(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    particle_layout: &wgpu::BindGroupLayout,
  ) -> ParticleBitonicSorter {
    let module = device.create_shader_module(wgpu::include_wgsl!("bitonic-sorter.wgsl"));
    let module = &module;
    let layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
      label: Some("BitonicSorter layout"),
      bind_group_layouts: &[particle_layout],
      push_constant_ranges: &[/*PushConstantRange {
        stages: wgpu::ShaderStages::COMPUTE,
        range: 0..3,
      }*/],
    });
    let layout = Some(&layout);
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
    ParticleBitonicSorter {
      flip_local,
      disperse_local,
    }
  }
  fn sort_1024(&self, encoder: &mut wgpu::CommandEncoder, particles: &wgpu::BindGroup) {
    let mut pass = encoder.begin_compute_pass(&ComputePassDescriptor {
      label: Some("sort1024"),
      timestamp_writes: None,
    });
    pass.set_pipeline(&self.flip_local);
    pass.set_bind_group(0, particles, &[]);
    pass.dispatch_workgroups(1, 1, 1);
  }
  pub fn set_buffer(&self, device: &wgpu::Device, buffer: &wgpu::Buffer) {
    todo!()
  }
  pub fn sort(&self, encoder: &mut wgpu::CommandEncoder, particles: &wgpu::BindGroup) {
    {
      let mut pass = encoder.begin_compute_pass(&ComputePassDescriptor {
        label: Some("flip local CP"),
        timestamp_writes: None,
      });
      pass.set_push_constants(0, &0u32.to_ne_bytes());
      // pass.dis
    }
  }
}
