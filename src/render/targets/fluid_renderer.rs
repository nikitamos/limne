use cw::with;
use std::marker::PhantomData;

use wgpu::{
  vertex_attr_array, BindGroup, Color, DepthBiasState, DepthStencilState, Extent3d,
  MultisampleState, RenderPassColorAttachment, RenderPassDepthStencilAttachment,
  RenderPassDescriptor, RenderPipeline, RenderPipelineDescriptor, StencilFaceState, StencilState,
  TextureUsages, VertexBufferLayout,
};

use crate::{
  math::sph_solver::Particle,
  render::{
    render_target::{ExternalResources, RenderTarget},
    texture_provider::{TextureProvider, TextureProviderDescriptor},
  },
};

use super::show_texture::TextureDrawer;

const PARTICLE_POS_BUFFER_LAYOUT: VertexBufferLayout = VertexBufferLayout {
  array_stride: std::mem::size_of::<Particle>() as u64,
  step_mode: wgpu::VertexStepMode::Instance,
  attributes: &vertex_attr_array![0 => Float32x3],
};

pub struct FluidRenderer {
  spheres_zbuf: TextureProvider,
  zbuf_smoothed: TextureProvider,
  thickness: TextureProvider,
  sphere_tex: TextureProvider,
  normals: TextureProvider,
  sphere_render: RenderPipeline,
  zbuf_smoother: TextureDrawer,
  merger: RenderPipeline,
}

pub struct FluidRendererResources<'a> {
  global_bg: &'a BindGroup,
}

pub struct FluidRenderInit {
  pub size: egui::Vec2,
}

impl<'a> ExternalResources<'a> for FluidRendererResources<'a> {}

impl<'a> RenderTarget<'a> for FluidRenderer {
  type RenderResources = FluidRendererResources<'a>;

  type InitResources = FluidRenderInit;

  type UpdateResources = Self::RenderResources;

  fn init(
    device: &wgpu::Device,
    _queue: &wgpu::Queue,
    _resources: &'a Self::RenderResources,
    _format: &wgpu::TextureFormat,
    init_res: Self::InitResources,
  ) -> Self
  where
    Self: Sized,
  {
    let tex_size = Extent3d {
      width: init_res.size.x as u32,
      height: init_res.size.y as u32,
      depth_or_array_layers: 1,
    };
    let mut desc = TextureProviderDescriptor {
      label: Some("shapes_zbuf".to_string()),
      size: tex_size,
      mip_level_count: 1,
      sample_count: 1,
      dimension: wgpu::TextureDimension::D2,
      format: wgpu::TextureFormat::Depth32Float,
      usage: wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::RENDER_ATTACHMENT,
      view_formats: vec![],
    };
    let spheres_zbuf = TextureProvider::new(device, desc.clone());

    desc = with!(desc: label = Some("zbuf_smoothed".to_owned()), usage = TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING);
    let zbuf_smoothed = TextureProvider::new(device, desc.clone());
    desc = with!(desc: label = Some("normals".to_owned()));
    let normals = TextureProvider::new(device, desc.clone());
    desc = with!(desc: label = Some("thickness".to_owned()));
    let thickness = TextureProvider::new(device, desc.clone());
    desc = with!(desc: label = Some("sphere_tex".to_owned()));
    let sphere_tex = TextureProvider::new(device, desc);

    // TODO: Copy from `simulation.rs`
    let sphere_render = device.create_render_pipeline(&RenderPipelineDescriptor {
      label: Some("Sphere render"),
      layout: todo!(),
      vertex: todo!(),
      primitive: todo!(),
      depth_stencil: Some(DepthStencilState {
        format: wgpu::TextureFormat::Depth32Float,
        depth_write_enabled: true,
        depth_compare: wgpu::CompareFunction::LessEqual,
        stencil: StencilState {
          front: StencilFaceState::IGNORE,
          back: StencilFaceState::IGNORE,
          read_mask: 0,
          write_mask: 0,
        },
        bias: DepthBiasState {
          constant: 0,
          slope_scale: 0.0,
          clamp: 0.0,
        },
      }),
      multisample: MultisampleState {
        count: 1,
        mask: !0,
        alpha_to_coverage_enabled: false,
      },
      fragment: todo!(),
      multiview: None,
      cache: None,
    });

    Self {
      spheres_zbuf,
      zbuf_smoothed,
      thickness,
      sphere_tex,
      normals,
      sphere_render,
      zbuf_smoother: todo!(),
      merger: todo!(),
    }
  }

  fn update(
    &mut self,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    resources: &'a Self::UpdateResources,
    encoder: &mut wgpu::CommandEncoder,
  ) {
    {
      let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
        label: Some("Spheres"),
        color_attachments: &[Some(RenderPassColorAttachment {
          view: &self.sphere_tex,
          resolve_target: None,
          ops: wgpu::Operations {
            load: wgpu::LoadOp::Clear(Color::WHITE),
            store: wgpu::StoreOp::Store,
          },
        })],
        depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
          view: &self.spheres_zbuf,
          depth_ops: Some(wgpu::Operations {
            load: wgpu::LoadOp::Clear(0.0),
            store: wgpu::StoreOp::Store,
          }),
          stencil_ops: None,
        }),
        timestamp_writes: None,
        occlusion_query_set: None,
      });
      pass.set_pipeline(todo!());
      pass.set_bind_group(0, resources.global_bg, &[]);
    }
  }

  fn resized(
    &mut self,
    device: &wgpu::Device,
    new_size: egui::Vec2,
    _resources: &'a Self::UpdateResources,
    _format: wgpu::TextureFormat,
  ) {
    let new_size = Extent3d {
      width: new_size.x as u32,
      height: new_size.y as u32,
      depth_or_array_layers: 1,
    };
    self.normals.resize(device, new_size);
    self.spheres_zbuf.resize(device, new_size);
    self.sphere_tex.resize(device, new_size);
    self.thickness.resize(device, new_size);
    // self.sphere_renderer.resized(device, texture);
  }

  fn render_into_pass(&self, pass: &mut wgpu::RenderPass, resources: &'a Self::RenderResources) {
    todo!()
  }
}
