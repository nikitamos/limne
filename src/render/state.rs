use bindings::{GLOBAL_BIND_LOC, GLOBAL_BIND_SIZE};
use egui_wgpu::{CallbackTrait, RenderState};
use std::{num::NonZero, sync::Arc};
use wgpu::{
  BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor,
  BindGroupLayoutEntry, Buffer, BufferBinding, BufferDescriptor, BufferUsages,
  CommandEncoderDescriptor, Device, PipelineLayoutDescriptor, RenderPipeline,
  RenderPipelineDescriptor, ShaderStages, TextureFormat, VertexState,
};

use crate::render::simulation::two_d::DefaultSim;

use super::simulation::{two_d, AsBuffer, Simulation};

pub(super) struct ClearPassState {
  clear_pipeline: RenderPipeline,
  simulation: two_d::DefaultSim,
  global_layout: BindGroupLayout,
  global_bind: BindGroup,
  viewport_buf: Buffer,
}

pub mod bindings {
  pub const GLOBAL_BIND_LOC: u32 = 0;
  pub const GLOBAL_BIND_SIZE: u64 = 4u64 * std::mem::size_of::<f32>() as u64;
}

/// This structure is responsible for storing WGPU resources for the clear pass
impl ClearPassState {
  pub fn update(&mut self, dt: f32, total: f32) {
    self.simulation.step(dt);
  }
  pub fn create(rstate: &RenderState) -> Self {
    let RenderState {
      device,
      adapter,
      queue,
      target_format: format,
      ..
    } = rstate;

    println!(
      "Adapter: {}\n Backend: {}",
      adapter.get_info().name,
      adapter.get_info().backend.to_str().to_uppercase()
    );

    let viewport_buf = device.create_buffer(&BufferDescriptor {
      label: None,
      size: GLOBAL_BIND_SIZE,
      usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
      mapped_at_creation: false,
    });

    let global_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
      label: Some("Global Binding Group"),
      entries: &[BindGroupLayoutEntry {
        binding: GLOBAL_BIND_LOC,
        visibility: ShaderStages::all(),
        ty: wgpu::BindingType::Buffer {
          ty: wgpu::BufferBindingType::Uniform,
          has_dynamic_offset: false,
          min_binding_size: NonZero::new(GLOBAL_BIND_SIZE),
        },
        count: None,
      }],
    });
    let global_bind = device.create_bind_group(&BindGroupDescriptor {
      label: Some("Global Bind Group"),
      layout: &global_layout,
      entries: &[BindGroupEntry {
        binding: GLOBAL_BIND_LOC,
        resource: wgpu::BindingResource::Buffer(BufferBinding {
          buffer: &viewport_buf,
          offset: 0,
          size: NonZero::new(GLOBAL_BIND_SIZE),
        }),
      }],
    });
    let clear_pipeline = Self::create_pipeline(&device, *format, &[&global_layout]);

    Self {
      clear_pipeline,
      simulation: DefaultSim::create_fully_initialized(
        65000,
        device,
        crate::render::simulation::PhysicalSize {
          width: 1200,
          height: 800,
        },
        *format,
        &global_layout,
      ),
      global_bind,
      viewport_buf,
      global_layout,
    }
  }

  fn create_pipeline(
    device: &Device,
    format: TextureFormat,
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
          format,
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

  // pub fn resize(&mut self, size: PhysicalSize<u32>) {
  // if size.width > 0 && size.height > 0 {
  //   self.config.width = size.width;
  //   self.config.height = size.height;
  //   self.surface.configure(&self.device, &self.config);
  //   self.simulation.as_mut().map(|s| {
  //     s.on_surface_resized(size, &self.device);
  //     s.reinit_pipelines(&self.device, self.config.format, &self.global_layout);
  //   });
  // }
  // }

  // pub fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
  //   let commands = std::iter::once(encoder.finish());
  //   let a = self.simulation.as_mut().map(|sim| {
  //     let sim_encoder = self
  //       .device
  //       .create_command_encoder(&CommandEncoderDescriptor {
  //         label: sim.encoder_label(),
  //       });
  //     sim.write_buffers(&self.queue);
  //     sim.run_passes(sim_encoder, &self.global_bind, &view)
  //   });

  //   self.queue.submit(commands.chain(a));
  //   output.present();

  //   Ok(())
  // }

  // pub fn set_simulation(&mut self, sim: Box<dyn Simulation>) -> Option<Box<dyn Simulation>> {
  //   let mut sim = sim;
  //   sim.init_pipelines(&self.device, self.config.format, &self.global_layout);
  //   self.simulation.replace(sim)
  // }
}

pub(crate) struct StateCallback {
  pub dt: f32,
  pub time: f32,
}

impl CallbackTrait for StateCallback {
  fn paint(
    &self,
    _info: egui::PaintCallbackInfo,
    pass: &mut wgpu::RenderPass<'static>,
    callback_resources: &egui_wgpu::CallbackResources,
  ) {
    let Some(state) = callback_resources.get::<ClearPassState>() else {
      unreachable!()
    };
    state.simulation.render_into_pass(&state.global_bind, pass);
  }

  fn prepare(
    &self,
    _device: &wgpu::Device,
    queue: &wgpu::Queue,
    screen_descriptor: &egui_wgpu::ScreenDescriptor,
    _encoder: &mut wgpu::CommandEncoder,
    callback_resources: &mut egui_wgpu::CallbackResources,
  ) -> Vec<wgpu::CommandBuffer> {
    // UPDATE/COMPUTE goes here?
    let Some(state) = callback_resources.get::<ClearPassState>() else {
      unreachable!()
    };
    queue.write_buffer(
      &state.viewport_buf,
      0,
      [
        screen_descriptor.size_in_pixels[0] as f32,
        screen_descriptor.size_in_pixels[0] as f32,
        self.time,
        self.dt,
      ]
      .as_bytes_buffer(),
    );
    let Some(state) = callback_resources.get_mut::<ClearPassState>() else {
      unreachable!()
    };
    state.simulation.write_buffers(queue);
    Vec::new()
  }

  fn finish_prepare(
    &self,
    _device: &wgpu::Device,
    _queue: &wgpu::Queue,
    egui_encoder: &mut wgpu::CommandEncoder,
    callback_resources: &mut egui_wgpu::CallbackResources,
  ) -> Vec<wgpu::CommandBuffer> {
    let Some(state) = callback_resources.get_mut::<ClearPassState>() else {
      unreachable!()
    };
    state.simulation.compute(egui_encoder, &state.global_bind);

    Vec::new()
  }
}
