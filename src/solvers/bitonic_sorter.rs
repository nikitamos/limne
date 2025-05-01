use wgpu::{core::device::queue, ComputePipelineDescriptor};

mod test {
  use wgpu::{InstanceDescriptor, RequestAdapterOptions};

  async fn setup_wgpu() -> Option<(wgpu::Device, wgpu::Queue)> {
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
      .await?;
    adapter
      .request_device(&wgpu::DeviceDescriptor::default(), None)
      .await
      .ok()
  }

  #[tokio::test]
  async fn test_1() {}
}

fn sort_256_cpu(a: &[i32]) {
  assert_eq!(a.len(), 256);
  for i in 0..8 { // sort passes & distances
    // swaps than can be done in parallel
    for i in 0..128 {
      
    }
    // Barrier here?
  }
}

pub struct BitonicSorter {
  pipeline: wgpu::ComputePipeline,
}

impl BitonicSorter {
  pub fn new(device: &wgpu::Device, queue: &wgpu::Queue) -> BitonicSorter {
    let pipeline = device.create_compute_pipeline(&ComputePipelineDescriptor {
      label: todo!(),
      layout: todo!(),
      module: todo!(),
      entry_point: todo!(),
      compilation_options: todo!(),
      cache: todo!(),
    });
    todo!()
  }
  fn sort_256(&self) {
    todo!()
  }
}
