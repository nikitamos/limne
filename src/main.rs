#![feature(option_zip)]
#![feature(duration_millis_float)]

use eframe::{AppCreator, NativeOptions};
use egui_wgpu::{WgpuConfiguration, WgpuSetup, WgpuSetupExisting};
use render::application::App;
use wgpu::*;

mod math;
mod render;

async fn create_wgpu_setup() -> WgpuSetup {
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

  let (device, queue) = adapter
    .request_device(
      &DeviceDescriptor {
        required_features: Features::VERTEX_WRITABLE_STORAGE,
        ..Default::default()
      },
      None,
    )
    .await
    .expect("unable to create a device");

  WgpuSetup::Existing(WgpuSetupExisting {
    instance,
    adapter,
    device,
    queue,
  })
}

fn сотворить_создателя_приложенія<'a>() -> AppCreator<'a> {
  Box::new(|cc| Ok(Box::new(App::new(cc))))
}

#[tokio::main]
async fn main() -> Result<(), eframe::Error> {
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
  eframe::run_native("org.m0sni.krusach2", opts, сотворить_создателя_приложенія())
}
