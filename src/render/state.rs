use std::sync::Arc;
use wgpu::{
  Backends, Color, CommandEncoderDescriptor, Device, Instance, InstanceDescriptor, Operations,
  Queue, RenderPassColorAttachment, RenderPassDescriptor, RequestAdapterOptions, Surface,
  SurfaceConfiguration, TextureViewDescriptor,
};
use winit::{dpi::PhysicalSize, window::Window};

pub(super) struct State<'a> {
  instance: Instance,
  surface: Surface<'a>,
  device: Device,
  queue: Queue,
  config: SurfaceConfiguration,
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
    surface.configure(&device, &config);

    println!("Using adapter {}", adapter.get_info().name);
    Self {
      instance,
      surface,
      device,
      queue,
      config,
    }
  }

  pub fn resize(&mut self, size: PhysicalSize<u32>) {
    if size.width > 0 && size.height > 0 {
      self.config.width = size.width;
      self.config.height = size.height;
      self.surface.configure(&self.device, &self.config);
    }
  }

  pub fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
    let output = self.surface.get_current_texture()?;
    let view = output.texture.create_view(&TextureViewDescriptor {
      ..Default::default()
    });

    let mut encoder = self
      .device
      .create_command_encoder(&CommandEncoderDescriptor {
        label: Some("Render Encoder"),
      });

    encoder.begin_render_pass(&RenderPassDescriptor {
      label: Some("Clear Pass"),
      color_attachments: &[Some(RenderPassColorAttachment {
        view: &view,
        resolve_target: None,
        ops: Operations {
          load: wgpu::LoadOp::Clear(Color::RED),
          store: wgpu::StoreOp::Store,
        },
      })],
      depth_stencil_attachment: None,
      timestamp_writes: None,
      occlusion_query_set: None,
    });

    self.queue.submit(std::iter::once(encoder.finish()));
    output.present();

    Ok(())
  }
}
