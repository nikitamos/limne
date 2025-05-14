use eframe::{AppCreator, NativeOptions};
use egui_wgpu::WgpuConfiguration;

use limne::create_wgpu_setup;
use limne::render::application::App;

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
