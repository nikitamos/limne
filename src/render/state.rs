use bindings::{GLOBAL_BIND_LOC, GLOBAL_BIND_SIZE};
use cgmath::Matrix4;
use egui_wgpu::{CallbackTrait, RenderState};
use std::num::NonZero;
use wgpu::{
  BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor,
  BindGroupLayoutEntry, Buffer, BufferBinding, BufferDescriptor, BufferUsages, DepthBiasState,
  ShaderStages, StencilState, TextureFormat, TextureUsages,
};

use crate::render::{
  render_target::RenderTarget, simulation::two_d::DefaultSim, targets::gizmo::GizmoResources,
};

use super::{
  simulation::{two_d, Simulation, SimulationParams, SimulationRegenOptions},
  targets::gizmo::Gizmo,
  AsBuffer,
};

pub(super) struct PersistentState {
  simulation: two_d::DefaultSim,
  global_layout: BindGroupLayout,
  global_bind: BindGroup,
  viewport_buf: Buffer,
  size: egui::Vec2,
  format: TextureFormat,
  projection: Matrix4<f32>,
  depth_texture: wgpu::Texture,
  depth_state: wgpu::DepthStencilState,
  gizmo: Gizmo,
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
    // TODO: kill
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

    let gizmo = Gizmo::init(
      device,
      &rstate.queue,
      &GizmoResources {
        global_layout: &global_layout,
        global_group: &global_bind,
      },
      format,
    );

    let depth_texture = device.create_texture(&wgpu::TextureDescriptor {
      label: Some("Depth texture"),
      size: wgpu::Extent3d {
        width: 128,
        height: 128,
        depth_or_array_layers: 1,
      },
      mip_level_count: 1,
      sample_count: 1,
      dimension: wgpu::TextureDimension::D2,
      format: TextureFormat::Depth32Float,
      usage: TextureUsages::RENDER_ATTACHMENT,
      view_formats: &[TextureFormat::Depth32Float],
    });

    let depth_state = wgpu::DepthStencilState {
      format: TextureFormat::Depth32FloatStencil8,
      depth_write_enabled: true,
      depth_compare: wgpu::CompareFunction::Less,
      stencil: StencilState {
        front: wgpu::StencilFaceState::IGNORE,
        back: wgpu::StencilFaceState::IGNORE,
        read_mask: 0,
        write_mask: 0,
      },
      bias: DepthBiasState {
        constant: 1,
        slope_scale: 1.,
        clamp: 1.0,
      },
    };

    Self {
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
      gizmo,
      depth_texture,
      depth_state
    }
  }

  pub fn resize(&mut self, size: egui::Vec2, device: &wgpu::Device) {
    if size.x > 0. && size.y > 0. {
      self.size = size;
      self.simulation.on_surface_resized(size, device);
      self
        .simulation
        .reinit_pipelines(device, self.format, &self.global_layout);
      self.projection = GL_TRANSFORM_TO_WGPU * cgmath::ortho(-size.x/2., size.x/2., -size.y/2., size.y/2., 0., 100000.0);
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
    state.gizmo.render_into_pass(
      pass,
      &GizmoResources {
        global_group: &state.global_bind,
        global_layout: &state.global_layout,
      },
    );
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
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    egui_encoder: &mut wgpu::CommandEncoder,
    callback_resources: &mut egui_wgpu::CallbackResources,
  ) -> Vec<wgpu::CommandBuffer> {
    let Some(state) = callback_resources.get_mut::<PersistentState>() else {
      unreachable!()
    };
    state.simulation.compute(egui_encoder, &state.global_bind);
    state
      .gizmo
      .update(device, queue, &state.global_bind, egui_encoder);

    Vec::new()
  }
}
