#![feature(option_zip)]
#![feature(duration_millis_float)]
#![feature(associated_type_defaults)]
#![feature(more_float_constants)]
#![feature(more_qualified_paths)]

use eframe::{AppCreator, NativeOptions};
use egui_wgpu::{WgpuConfiguration, WgpuSetup, WgpuSetupExisting};
use render::application::App;
use wgpu::*;

mod render;
mod solvers;

async fn create_wgpu_setup() -> WgpuSetup {
  let required_limits = Limits {
    max_bind_groups: 5,
    max_compute_invocations_per_workgroup: 1024,
    max_compute_workgroup_storage_size: 49152,
    ..Default::default()
  };

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

  log::info!("Adapter: {}", adapter.get_info().name);
  log::info!("Backend: {}", adapter.get_info().backend);

  log::debug!("Required limits:\n {:#?}", required_limits);
  log::debug!("Adapter's limits:\n{:#?}", adapter.limits());

  let (device, queue) = adapter
    .request_device(
      &DeviceDescriptor {
        required_features: Features::VERTEX_WRITABLE_STORAGE | Features::POLYGON_MODE_LINE,
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
