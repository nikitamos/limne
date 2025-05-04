#![feature(option_zip)]
#![feature(duration_millis_float)]
#![feature(associated_type_defaults)]
#![feature(more_float_constants)]
#![feature(more_qualified_paths)]

use eframe::{AppCreator, NativeOptions};
use egui_wgpu::{WgpuConfiguration, WgpuSetup, WgpuSetupExisting};
use render::application::App;
use solvers::{
  bitonic_sorter::{LOCAL_ARRAY_SIZE, LOCAL_PASS_SIZE},
  sph_solver_gpu::Particle,
};
use wgpu::*;

mod render;
mod solvers;

async fn create_wgpu_setup() -> WgpuSetup {
  let required_limits = Limits {
    max_bind_groups: 5,
    max_compute_invocations_per_workgroup: LOCAL_PASS_SIZE,
    max_compute_workgroup_storage_size: LOCAL_ARRAY_SIZE * size_of::<Particle>() as u32,
    max_push_constant_size: 8,
    ..Default::default()
  };
  let required_features =
    Features::VERTEX_WRITABLE_STORAGE | Features::POLYGON_MODE_LINE | Features::PUSH_CONSTANTS;

  log::info!("Required workgroup size: {LOCAL_PASS_SIZE}");
  log::info!(
    "Required local storage: {}",
    required_limits.max_compute_workgroup_storage_size
  );

  let instance = wgpu::Instance::new(&InstanceDescriptor {
    backends: Backends::all(),
    ..Default::default()
  });

  let adapter = instance
    .request_adapter(&RequestAdapterOptions {
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
      &DeviceDescriptor {
        required_features,
        required_limits,
        ..Default::default()
      },
      None,
    )
    .await
    .expect("Your hardware is unsupported");

  WgpuSetup::Existing(WgpuSetupExisting {
    instance,
    adapter,
    device,
    queue,
  })
}

fn make_app_creator<'a>() -> AppCreator<'a> {
  Box::new(|cc| Ok(Box::new(App::new(cc))))
}

#[tokio::main]
async fn main() -> Result<(), eframe::Error> {
  env_logger::init();
  let opts = NativeOptions {
    hardware_acceleration: eframe::HardwareAcceleration::Required,
    renderer: eframe::Renderer::Wgpu,
    run_and_return: false,
    centered: true,
    wgpu_options: WgpuConfiguration {
      wgpu_setup: create_wgpu_setup().await,
      ..Default::default()
    },
    ..Default::default()
  };
  eframe::run_native("m0sni.limne", opts, make_app_creator())
}
