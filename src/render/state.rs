use bindings::{GLOBAL_BIND_LOC, GLOBAL_BIND_SIZE};
use cgmath::{Deg, Matrix4};
use egui_wgpu::{CallbackTrait, RenderState};
use std::num::NonZero;
use wgpu::{
  BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor,
  BindGroupLayoutEntry, Buffer, BufferBinding, BufferDescriptor, BufferUsages, Color,
  DepthBiasState, Operations, RenderPassColorAttachment, RenderPassDepthStencilAttachment,
  RenderPassDescriptor, ShaderStages, StencilState, TextureFormat, TextureUsages,
};

use crate::render::{
  render_target::RenderTarget,
  targets::{gizmo::GizmoResources, show_texture::TexDrawResources, simulation::*},
  texture_provider::TextureProviderDescriptor,
};

use super::{
  targets::{gizmo::Gizmo, show_texture::TextureDrawer},
  texture_provider::TextureProvider,
  AsBuffer,
};

pub(super) struct PersistentState {
  simulation: SphSimulation,
  global_layout: BindGroupLayout,
  global_bind: BindGroup,
  global_buf: Buffer,
  size: egui::Vec2,
  format: TextureFormat,
  projection: Matrix4<f32>,
  depth_texture: TextureProvider,
  target_texture: TextureProvider,
  depth_state: wgpu::DepthStencilState,
  gizmo: Gizmo,
  texture_drawer: TextureDrawer,
}

pub mod bindings {
  use cgmath::Matrix4;

  pub const GLOBAL_BIND_LOC: u32 = 0;
  pub const GLOBAL_BIND_SIZE: u64 =
    (4 * std::mem::size_of::<f32>() + 2*std::mem::size_of::<Matrix4<f32>>()) as u64;
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
  pub fn create(rstate: &RenderState, opts: SimulationRegenOptions) -> Self {
    let RenderState {
      device,
      adapter,
      target_format: format,
      queue,
      ..
    } = rstate;

    println!(
      "Adapter: {}\nBackend: {}",
      adapter.get_info().name,
      adapter.get_info().backend.to_str().to_uppercase()
    );

    let global_buf = device.create_buffer(&BufferDescriptor {
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
          buffer: &global_buf,
          offset: 0,
          size: NonZero::new(GLOBAL_BIND_SIZE),
        }),
      }],
    });

    let depth_texture = TextureProvider::new(
      &device,
      TextureProviderDescriptor {
        label: Some("Depth texture".into()),
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
        view_formats: vec![TextureFormat::Depth32Float],
      },
    );
    let target_texture = TextureProvider::new(
      &device,
      TextureProviderDescriptor {
        label: Some("Target texture".to_owned()),
        size: wgpu::Extent3d {
          width: 10,
          height: 10,
          depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: *format,
        usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
        view_formats: vec![*format],
      },
    );

    let texture_drawer = TextureDrawer::init(
      device,
      queue,
      &TexDrawResources {
        texture: &target_texture,
      },
      format,
      (),
    );

    let depth_stencil = wgpu::DepthStencilState {
      format: TextureFormat::Depth32Float,
      depth_write_enabled: true,
      depth_compare: wgpu::CompareFunction::LessEqual,
      stencil: StencilState {
        front: wgpu::StencilFaceState::IGNORE,
        back: wgpu::StencilFaceState::IGNORE,
        read_mask: 0,
        write_mask: 0,
      },
      bias: DepthBiasState {
        constant: 0,
        slope_scale: 0.0,
        clamp: 0.0,
      },
    };
    let gizmo = Gizmo::init(
      device,
      queue,
      &GizmoResources {
        global_layout: &global_layout,
        global_group: &global_bind,
        depth_stencil: &depth_stencil,
      },
      format,
      (),
    );

    Self {
      simulation: SphSimulation::init(
        device,
        queue,
        &SimResources {
          params: &Default::default(),
          global_group: &global_bind,
          global_layout: &global_layout,
          regen_options: Some(opts),
          depth_stencil: &depth_stencil,
        },
        format,
        SimInit {
          count: 5000,
          size: egui::Vec2 {
            x: 1200.0,
            y: 800.0,
          },
          depth_state: &depth_stencil,
        },
      ),
      global_bind,
      global_buf,
      global_layout,
      size: egui::Vec2::ZERO,
      format: *format,
      projection: cgmath::ortho(0.0, 200.0, 0.0, 400.0, 0.0, 30.0),
      gizmo,
      target_texture,
      depth_texture,
      depth_state: depth_stencil,
      texture_drawer,
    }
  }

  fn resize(&mut self, size: egui::Vec2, device: &wgpu::Device, callback: &StateCallback) {
    if size.x > 0. && size.y > 0. {
      self.size = size;
      let new_tex_size = wgpu::Extent3d {
        width: size.x as u32,
        height: size.y as u32,
        depth_or_array_layers: 1,
      };
      self.target_texture.resize(device, new_tex_size);
      self.depth_texture.resize(device, new_tex_size);
      self.simulation.resized(
        device,
        size,
        &SimUpdateResources {
          params: &callback.params,
          global_group: &self.global_bind,
          global_layout: &self.global_layout,
          depth_stencil: &self.depth_state,
          dt: callback.dt,
        },
        self.format,
      );
      self.texture_drawer.resized(device, &self.target_texture);

      let s = size.x.min(size.y);
      self.projection = GL_TRANSFORM_TO_WGPU * cgmath::perspective(Deg(60.0), size.x/size.y, 100., 10000.);
      // self.projection = GL_TRANSFORM_TO_WGPU
      //   * cgmath::ortho(
      //     -size.x / 2.,
      //     size.x / 2.,
      //     -size.y / 2.,
      //     size.y / 2.,
      //     -1000.0,
      //     10000.0,
      //   );
    }
  }

  /// If `size` is different from stored internally,
  /// calls [`Self::resize`]
  pub fn check_resize(
    &mut self,
    size: egui::Vec2,
    device: &wgpu::Device,
    callback: &StateCallback,
  ) {
    if size != self.size {
      self.resize(size, device, callback);
    }
  }
}

pub(crate) struct StateCallback {
  pub dt: f32,
  pub time: f32,
  pub regen_opts: Option<SimulationRegenOptions>,
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
    // Renderig the target texture to the viewport goes here
    let Some(state) = callback_resources.get::<PersistentState>() else {
      unreachable!()
    };
    state.texture_drawer.render_into_pass(
      pass,
      &TexDrawResources {
        texture: &state.target_texture,
      },
    );
  }

  fn prepare(
    &self,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    screen_descriptor: &egui_wgpu::ScreenDescriptor,
    encoder: &mut wgpu::CommandEncoder,
    callback_resources: &mut egui_wgpu::CallbackResources,
  ) -> Vec<wgpu::CommandBuffer> {
    // UPDATE goes here
    let Some(state) = callback_resources.get_mut::<PersistentState>() else {
      unreachable!()
    };
    let size = egui::Vec2 {
      x: screen_descriptor.size_in_pixels[0] as f32,
      y: screen_descriptor.size_in_pixels[1] as f32,
    };
    state.check_resize(size, device, self);

    let buf_vec: Vec<u8> = [size.x, size.y, self.time, self.dt]
      .as_bytes_buffer()
      .to_owned()
      .into_iter()
      .chain(self.camera.as_bytes_buffer().to_owned())
      .chain(state.projection.as_bytes_buffer().to_owned())
      .collect();
    queue.write_buffer(&state.global_buf, 0, &buf_vec);

    state.simulation.update(
      device,
      queue,
      &SimUpdateResources {
        params: &self.params,
        depth_stencil: &state.depth_state,
        global_group: &state.global_bind,
        global_layout: &state.global_layout,
        dt: self.dt,
      },
      encoder,
    );
    Vec::new()
  }

  fn finish_prepare(
    &self,
    _device: &wgpu::Device,
    _queue: &wgpu::Queue,
    egui_encoder: &mut wgpu::CommandEncoder,
    callback_resources: &mut egui_wgpu::CallbackResources,
  ) -> Vec<wgpu::CommandBuffer> {
    // Render to the `target_texture` goes here.
    let Some(state) = callback_resources.get_mut::<PersistentState>() else {
      unreachable!()
    };
    {
      let mut pass = egui_encoder.begin_render_pass(&RenderPassDescriptor {
        label: Some("Target pass"),
        color_attachments: &[Some(RenderPassColorAttachment {
          view: &state.target_texture,
          resolve_target: None,
          ops: Operations {
            load: wgpu::LoadOp::Clear(Color::WHITE),
            store: wgpu::StoreOp::Store,
          },
        })],
        depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
          view: &state.depth_texture,
          depth_ops: Some(Operations {
            load: wgpu::LoadOp::Clear(1.0),
            store: wgpu::StoreOp::Store,
          }),
          stencil_ops: None,
        }),
        timestamp_writes: None,
        occlusion_query_set: None,
      });
      state.gizmo.render_into_pass(
        &mut pass,
        &GizmoResources {
          global_group: &state.global_bind,
          global_layout: &state.global_layout,
          depth_stencil: &state.depth_state,
        },
      );
      state.simulation.render_into_pass(
        &mut pass,
        &SimResources {
          params: &self.params,
          global_group: &state.global_bind,
          global_layout: &state.global_layout,
          regen_options: self.regen_opts,
          depth_stencil: &state.depth_state,
        },
      );
    }

    Vec::new()
  }
}
