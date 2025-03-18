use bindings::{VIEWPORT_SIZE_LOCATION, VIEWPORT_SIZE_SIZE};
use std::{num::NonZero, sync::Arc};
use wgpu::{
  Backends, BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout,
  BindGroupLayoutDescriptor, BindGroupLayoutEntry, Buffer, BufferBinding, BufferDescriptor,
  BufferUsages, Color, CommandEncoderDescriptor, Device, DeviceDescriptor, Features, Instance,
  InstanceDescriptor, Operations, PipelineLayoutDescriptor, Queue, RenderPassColorAttachment,
  RenderPassDescriptor, RenderPipeline, RenderPipelineDescriptor, RequestAdapterOptions,
  ShaderStages, Surface, SurfaceConfiguration, TextureViewDescriptor, VertexState,
};
use winit::{dpi::PhysicalSize, window::Window};

use super::simulation::{AsBuffer, Simulation};

pub(super) struct State<'a> {
  instance: Instance,
  surface: Surface<'a>,
  pub(super) device: Device,
  queue: Queue,
  config: SurfaceConfiguration,
  clear_pipeline: RenderPipeline,
  simulation: Option<Box<dyn Simulation>>,
  global_layout: BindGroupLayout,
  global_bind: BindGroup,
  viewport_buf: Buffer,
}

pub mod bindings {
  pub const GLOBAL_BIND_GROUP: u32 = 0;
  pub const VIEWPORT_SIZE_LOCATION: u32 = 0;
  pub const VIEWPORT_SIZE_SIZE: u64 = 2u64 * std::mem::size_of::<f32>() as u64;
}

impl<'a> State<'a> {
  pub fn update(&mut self, dt: f32) {
    self.simulation.as_mut().map(|s| s.step(dt));
  }
  pub async fn create(window: Arc<Window>) -> Self {
    let instance = wgpu::Instance::new(&InstanceDescriptor {
      backends: Backends::VULKAN,
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
      .request_device(&DeviceDescriptor {
        required_features: Features::BUFFER_BINDING_ARRAY | Features::STORAGE_RESOURCE_BINDING_ARRAY,
        ..Default::default()
      }, None)
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

    println!(
      "Adapter: {}\n Backend: {}",
      adapter.get_info().name,
      adapter.get_info().backend.to_str().to_uppercase()
    );

    let viewport_buf = device.create_buffer(&BufferDescriptor {
      label: None,
      size: VIEWPORT_SIZE_SIZE,
      usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
      mapped_at_creation: false,
    });

    let global_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
      label: Some("Global Binding Group"),
      entries: &[BindGroupLayoutEntry {
        binding: VIEWPORT_SIZE_LOCATION,
        visibility: ShaderStages::all(),
        ty: wgpu::BindingType::Buffer {
          ty: wgpu::BufferBindingType::Uniform,
          has_dynamic_offset: false,
          min_binding_size: NonZero::new(2u64 * std::mem::size_of::<f32>() as u64),
        },
        count: None,
      }],
    });
    let global_bind = device.create_bind_group(&BindGroupDescriptor {
      label: Some("Global Bind Group"),
      layout: &global_layout,
      entries: &[BindGroupEntry {
        binding: VIEWPORT_SIZE_LOCATION,
        resource: wgpu::BindingResource::Buffer(BufferBinding {
          buffer: &viewport_buf,
          offset: 0,
          size: NonZero::new(VIEWPORT_SIZE_SIZE),
        }),
      }],
    });
    let render_pipeline = Self::create_pipeline(&device, &config, &[&global_layout]);

    Self {
      instance,
      surface,
      device,
      queue,
      config,
      clear_pipeline: render_pipeline,
      simulation: None,
      global_bind,
      viewport_buf,
      global_layout,
    }
  }

  fn create_pipeline(
    device: &Device,
    config: &SurfaceConfiguration,
    layouts: &[&BindGroupLayout],
  ) -> RenderPipeline {
    let shader = device.create_shader_module(wgpu::include_wgsl!("shader.wgsl"));
    let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
      label: Some("Render pipeline layout"),
      bind_group_layouts: layouts,
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

      pass.set_pipeline(&self.clear_pipeline);
    }

    self.queue.write_buffer(
      &self.viewport_buf,
      0,
      [self.config.width as f32, self.config.height as f32].as_bytes_buffer(),
    );

    let commands = std::iter::once(encoder.finish());
    let a = self.simulation.as_ref().map(|sim| {
      let sim_encoder = self
        .device
        .create_command_encoder(&CommandEncoderDescriptor {
          label: sim.encoder_label(),
        });
      sim.write_buffers(&self.queue);
      sim.run_passes(sim_encoder, &self.global_bind, &view)
    });

    self.queue.submit(commands.chain(a));
    output.present();

    Ok(())
  }

  pub fn set_simulation(&mut self, sim: Box<dyn Simulation>) -> Option<Box<dyn Simulation>> {
    let mut sim = sim;
    sim.init_pipelines(&self.device, self.config.format, &self.global_layout);
    self.simulation.replace(sim)
  }
}
