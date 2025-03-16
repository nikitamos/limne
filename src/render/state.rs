use std::sync::Arc;
use wgpu::{
  Backends, Device, Instance, InstanceDescriptor, Queue, RequestAdapterOptions, Surface, SurfaceConfiguration
};
use winit::window::Window;


pub(super) struct State<'a> {
  instance: Instance,
  surface: Surface<'a>,
  device: Device,
  queue: Queue,
  config: SurfaceConfiguration
}

impl<'a> State<'a> {
  pub async fn create(window: Arc<Window>) -> Self {
    let instance = wgpu::Instance::new(&InstanceDescriptor {
      backends: Backends::PRIMARY,
      ..Default::default()
    });
    let surface = instance
      .create_surface(Arc::clone(&window))
      .expect("Unable to create a surface");

    let adapter = instance
      .request_adapter(&RequestAdapterOptions {
        compatible_surface: Some(&surface),
        power_preference: wgpu::PowerPreference::HighPerformance,
        force_fallback_adapter: false,
      })
      .await
      .expect("Unable to create an adapter");

    let (device, queue) = adapter
      .request_device(&Default::default(), None)
      .await
      .expect("unable to create a device");

    let surface_caps = surface.get_capabilities(&adapter);
    
    let surface_format = surface_caps
      .formats
      .iter()
      .find(|f| f.is_srgb())
      .copied()
      .unwrap_or(surface_caps.formats[0]);

    let config = wgpu::SurfaceConfiguration {
      usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
      format: surface_format,
      width: window.inner_size().width,
      height: window.inner_size().height,
      present_mode: surface_caps.present_modes[0],
      alpha_mode: surface_caps.alpha_modes[0],
      view_formats: vec![],
      desired_maximum_frame_latency: 2,
    };

    println!("Using adapter {}", adapter.get_info().name);

    Self {
      instance,
      surface,
      device,
      queue,
      config
    }
  }
}