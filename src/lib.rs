#![feature(option_zip)]
#![feature(duration_millis_float)]
#![feature(associated_type_defaults)]
#![feature(more_float_constants)]
#![feature(more_qualified_paths)]

use solvers::{
  bitonic_sorter::{LOCAL_ARRAY_SIZE, LOCAL_PASS_SIZE},
  sph_solver_gpu::Particle,
};
use wgpu::Features;
#[macro_export]
macro_rules! with {
  ($x:ident: $($($fields:ident).* = $val: expr), *) => {
      {
        let mut y = $x;
        $(y$(.$fields)* = $val;)*
        y
      }
  };
  ($x:expr => $($($fields:ident).* = $val: expr), *) => {
      {
        let mut y = $x;
        // TODO: Reuse arm #0
        $(y$(.$fields)* = $val;)*
        y
      }
  };
}
pub mod render;
pub mod solvers;

pub async fn create_wgpu_setup() -> egui_wgpu::WgpuSetup {
  let required_limits = wgpu::Limits {
    max_bind_groups: 5,
    max_compute_invocations_per_workgroup: LOCAL_PASS_SIZE,
    max_compute_workgroup_size_x: LOCAL_PASS_SIZE,
    max_compute_workgroup_storage_size: LOCAL_ARRAY_SIZE * size_of::<Particle>() as u32,
    max_push_constant_size: 8,
    ..Default::default()
  };
  let required_features = Features::VERTEX_WRITABLE_STORAGE
    | Features::POLYGON_MODE_LINE
    | Features::PUSH_CONSTANTS
    | Features::ADDRESS_MODE_CLAMP_TO_BORDER;

  log::info!("Required workgroup size: {LOCAL_PASS_SIZE}");
  log::info!(
    "Required local storage: {}",
    required_limits.max_compute_workgroup_storage_size
  );

  let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
    backends: wgpu::Backends::all(),
    ..Default::default()
  });

  let adapter = instance
    .request_adapter(&wgpu::RequestAdapterOptions {
      compatible_surface: None,
      power_preference: wgpu::PowerPreference::HighPerformance,
      force_fallback_adapter: false,
    })
    .await
    .expect("Unable to create an adapter");

  log::info!("Backend: {}", adapter.get_info().backend);
  log::info!("Adapter: {}", adapter.get_info().name);
  log::debug!("Adapter's limits:\n{:#?}", adapter.limits());

  let (device, queue) = adapter
    .request_device(
      &wgpu::DeviceDescriptor {
        required_features,
        required_limits,
        ..Default::default()
      },
      None,
    )
    .await
    .expect("Your hardware is unsupported");

  egui_wgpu::WgpuSetup::Existing(egui_wgpu::WgpuSetupExisting {
    instance,
    adapter,
    device,
    queue,
  })
}
