use bindings::{GLOBAL_BIND_LOC, GLOBAL_BIND_SIZE};
use cgmath::Matrix4;
use egui_wgpu::{CallbackTrait, RenderState};
use std::{io::Read, num::NonZero};
use wgpu::{
  BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor,
  BindGroupLayoutEntry, Buffer, BufferBinding, BufferDescriptor, BufferUsages, Device,
  PipelineLayoutDescriptor, RenderPipeline, RenderPipelineDescriptor, ShaderStages, TextureFormat,
  VertexState,
};

use crate::render::simulation::two_d::DefaultSim;

use super::simulation::{two_d, AsBuffer, Simulation, SimulationParams, SimulationRegenOptions};

pub(super) struct PersistentState {
  clear_pipeline: RenderPipeline,
  simulation: two_d::DefaultSim,
  global_layout: BindGroupLayout,
  global_bind: BindGroup,
  viewport_buf: Buffer,
  size: egui::Vec2,
  format: TextureFormat,
  projection: Matrix4<f32>,
}

pub mod bindings {
  use cgmath::Matrix4;

  pub const GLOBAL_BIND_LOC: u32 = 0;
  pub const GLOBAL_BIND_SIZE: u64 =
    (4 * std::mem::size_of::<f32>() + std::mem::size_of::<Matrix4<f32>>()) as u64;
}

#[rustfmt::skip]
pub const GL_TRANSFORM_TO_WGPU: Matrix4<f32> =
  Matrix4::new(
    1.0, 0.0, 0.0, 0.0,
    0.0, 1.0, 0.0, 0.0,
    0.0, 0.0, 0.5, 0.0,
    0.0, 0.0, 0.5, 1.0
);

/// This structure is responsible for storing WGPU resources for the clear pass
impl PersistentState {
  pub fn update(&mut self, dt: f32, total: f32) {
    self.simulation.step(dt);
  }
  pub fn create(rstate: &RenderState, opts: SimulationRegenOptions) -> Self {
    let RenderState {
      device,
      adapter,
      target_format: format,
      ..
    } = rstate;

    println!(
      "Adapter: {}\nBackend: {}",
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
    let clear_pipeline = Self::create_pipeline(device, *format, &[&global_layout]);

    Self {
      clear_pipeline,
      simulation: DefaultSim::create_fully_initialized(
        32000,
        device,
        egui::Vec2 {
          x: 1200.0,
          y: 800.0,
        },
        *format,
        &global_layout,
        opts,
      ),
      global_bind,
      viewport_buf,
      global_layout,
      size: egui::Vec2::ZERO,
      format: *format,
      projection: cgmath::ortho(0.0, 200.0, 0.0, 400.0, 0.0, 30.0),
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

    
    device.create_render_pipeline(&RenderPipelineDescriptor {
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
    })
  }

  pub fn resize(&mut self, size: egui::Vec2, device: &wgpu::Device) {
    if size.x > 0. && size.y > 0. {
      self.size = size;
      self.simulation.on_surface_resized(size, device);
      self
        .simulation
        .reinit_pipelines(device, self.format, &self.global_layout);
      self.projection = GL_TRANSFORM_TO_WGPU * cgmath::ortho(0., size.x, 0., size.y, 0., 100000.0);
    }
  }

  /// If `size` is different from stored internally,
  /// calls [`Self::resize`]
  pub fn check_resize(&mut self, size: egui::Vec2, device: &wgpu::Device) {
    if size != self.size {
      self.resize(size, device);
    }
  }
}

pub(crate) struct StateCallback {
  pub dt: f32,
  pub time: f32,
  pub regen_opts: Option<SimulationRegenOptions>,
  pub regen_pos: bool,
  pub params: SimulationParams,
  pub camera: Matrix4<f32>,
}

impl CallbackTrait for StateCallback {
  fn paint(
    &self,
    _info: egui::PaintCallbackInfo,
    pass: &mut wgpu::RenderPass<'static>,
    callback_resources: &egui_wgpu::CallbackResources,
  ) {
    let Some(state) = callback_resources.get::<PersistentState>() else {
      unreachable!()
    };
    state.simulation.render_into_pass(&state.global_bind, pass);
  }

  fn prepare(
    &self,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    screen_descriptor: &egui_wgpu::ScreenDescriptor,
    _encoder: &mut wgpu::CommandEncoder,
    callback_resources: &mut egui_wgpu::CallbackResources,
  ) -> Vec<wgpu::CommandBuffer> {
    // UPDATE/COMPUTE goes here?
    let Some(state) = callback_resources.get::<PersistentState>() else {
      unreachable!()
    };
    let size = egui::Vec2 {
      x: screen_descriptor.size_in_pixels[0] as f32,
      y: screen_descriptor.size_in_pixels[1] as f32,
    };

    let projection = state.projection * self.camera;

    let buf_vec: Vec<u8> = [size.x, size.y, self.time, self.dt]
      .as_bytes_buffer()
      .to_owned()
      .into_iter()
      .chain(projection.as_bytes_buffer().to_owned())
      .collect();
    queue.write_buffer(&state.viewport_buf, 0, &buf_vec);

    let Some(state) = callback_resources.get_mut::<PersistentState>() else {
      unreachable!()
    };
    state.simulation.set_params(self.params);
    state.check_resize(size, device);
    if let Some(opts) = self.regen_opts {
      state.simulation.regenerate_grid(device, opts);
    }
    if self.regen_pos {
      state.simulation.regenerate_positions(device);
    }
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
    let Some(state) = callback_resources.get_mut::<PersistentState>() else {
      unreachable!()
    };
    state.simulation.compute(egui_encoder, &state.global_bind);

    Vec::new()
  }
}
