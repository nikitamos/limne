use std::sync::Arc;
use wgpu::{
  Backends, Color, CommandEncoderDescriptor, Device, Instance, InstanceDescriptor, Operations,
  PipelineLayoutDescriptor, Queue, RenderPassColorAttachment, RenderPassDescriptor, RenderPipeline,
  RenderPipelineDescriptor, RequestAdapterOptions, Surface, SurfaceConfiguration,
  TextureViewDescriptor, VertexState,
};
use winit::{dpi::PhysicalSize, window::Window};

use super::simulation::Simulation;

pub(super) struct State<'a> {
  instance: Instance,
  surface: Surface<'a>,
  pub(super) device: Device,
  queue: Queue,
  config: SurfaceConfiguration,
  render_pipeline: RenderPipeline,
  simulation: Option<Box<dyn Simulation>>,
}

impl<'a> State<'a> {
  pub async fn create(window: Arc<Window>) -> Self {
    let instance = wgpu::Instance::new(&InstanceDescriptor {
      backends: Backends::all(),
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
    let render_pipeline = Self::create_pipeline(&device, &config);
    Self {
      instance,
      surface,
      device,
      queue,
      config,
      render_pipeline,
      simulation: None
    }
  }

  fn create_pipeline(device: &Device, config: &SurfaceConfiguration) -> RenderPipeline {
    let shader = device.create_shader_module(wgpu::include_wgsl!("shader.wgsl"));
    let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
      label: Some("Render pipeline layout"),
      bind_group_layouts: &[],
      push_constant_ranges: &[],
    });

    let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
      label: Some("random pipeline"),
      layout: Some(&pipeline_layout),
      vertex: VertexState {
        module: &shader,
        entry_point: Some("vs_main"),
        buffers: &[],
        compilation_options: Default::default(),
      },
      primitive: wgpu::PrimitiveState {
        topology: wgpu::PrimitiveTopology::TriangleList,
        strip_index_format: None,
        front_face: wgpu::FrontFace::Ccw,
        cull_mode: Some(wgpu::Face::Back),
        polygon_mode: wgpu::PolygonMode::Fill,
        unclipped_depth: false,
        conservative: false,
      },
      depth_stencil: None,
      multisample: wgpu::MultisampleState {
        count: 1,
        mask: !0,
        alpha_to_coverage_enabled: false,
      },
      fragment: Some(wgpu::FragmentState {
        module: &shader,
        entry_point: Some("fs_main"),
        targets: &[Some(wgpu::ColorTargetState {
          format: config.format,
          blend: Some(wgpu::BlendState::REPLACE),
          write_mask: wgpu::ColorWrites::ALL,
        })],
        compilation_options: wgpu::PipelineCompilationOptions::default(),
      }),
      multiview: None,
      cache: None,
    });
    pipeline
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

    {
      let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
        label: Some("Clear Pass"),
        color_attachments: &[Some(RenderPassColorAttachment {
          view: &view,
          resolve_target: None,
          ops: Operations {
            load: wgpu::LoadOp::Clear(Color::WHITE),
            store: wgpu::StoreOp::Store,
          },
        })],
        depth_stencil_attachment: None,
        timestamp_writes: None,
        occlusion_query_set: None,
      });

      pass.set_pipeline(&self.render_pipeline);
    }

    let commands = std::iter::once(encoder.finish());
    let a = self.simulation.as_ref().map(|sim| {
      let sim_encoder = self
        .device
        .create_command_encoder(&CommandEncoderDescriptor {
          label: sim.encoder_label(),
        });
      sim.run_passes(sim_encoder, &view)
    });

    self.queue.submit(commands.chain(a));
    output.present();

    Ok(())
  }

  pub fn set_simulation(&mut self, sim: Box<dyn Simulation>) -> Option<Box<dyn Simulation>> {
    let mut sim = sim;
    sim.init_pipelines(&self.device, self.config.format);
    self.simulation.replace(sim)
  }
}
