use limne::with;

use wgpu::{
  vertex_attr_array, BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout,
  BindGroupLayoutDescriptor, BindGroupLayoutEntry, Color, DepthBiasState, DepthStencilState,
  Extent3d, FragmentState, MultisampleState, RenderPassColorAttachment,
  RenderPassDepthStencilAttachment, RenderPassDescriptor, RenderPipeline, RenderPipelineDescriptor,
  ShaderStages, StencilFaceState, StencilState, TexelCopyTextureInfo, VertexBufferLayout,
};

use crate::{
  math::sph_solver_gpu::Particle,
  render::{
    render_target::{ExternalResources, RenderTarget},
    texture_provider::{TextureProvider, TextureProviderDescriptor},
  },
};

use super::show_texture::{TextureDrawer, TextureDrawerInitRes, TextureDrawerResources};

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
  merger: TextureDrawer,
  merge_bgl: BindGroupLayout,
  merge_bg: BindGroup,
}

pub struct FluidRendererResources<'a> {
  pub global_bg: &'a BindGroup,
  pub params_bg: &'a BindGroup,
  pub pos_buf: &'a wgpu::Buffer,
  pub count: u32,
}

pub struct FluidRenderInit<'a> {
  pub size: egui::Vec2,
  pub global_layout: &'a BindGroupLayout,
  pub params_layout: &'a BindGroupLayout,
  pub depth_stencil_state: DepthStencilState,
}

impl<'a> ExternalResources<'a> for FluidRendererResources<'a> {}

impl<'a> RenderTarget<'a> for FluidRenderer {
  type RenderResources = FluidRendererResources<'a>;
  type InitResources = FluidRenderInit<'a>;
  type UpdateResources = Self::RenderResources;

  fn update(
    &mut self,
    _device: &wgpu::Device,
    _queue: &wgpu::Queue,
    resources: &'a Self::UpdateResources,
    encoder: &mut wgpu::CommandEncoder,
  ) {
    {
      let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
        label: Some("Spheres"),
        color_attachments: &[
          Some(RenderPassColorAttachment {
            view: &self.sphere_tex,
            resolve_target: None,
            ops: wgpu::Operations {
              load: wgpu::LoadOp::Clear(Color::WHITE),
              store: wgpu::StoreOp::Store,
            },
          }),
          Some(RenderPassColorAttachment {
            view: &self.thickness,
            resolve_target: None,
            ops: wgpu::Operations {
              load: wgpu::LoadOp::Clear(Color::WHITE),
              store: wgpu::StoreOp::Store,
            },
          }),
        ],
        depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
          view: &self.spheres_zbuf,
          depth_ops: Some(wgpu::Operations {
            load: wgpu::LoadOp::Clear(1.0),
            store: wgpu::StoreOp::Store,
          }),
          stencil_ops: None,
        }),
        timestamp_writes: None,
        occlusion_query_set: None,
      });
      pass.set_pipeline(&self.sphere_render);
      pass.set_bind_group(0, resources.global_bg, &[]);
      pass.set_bind_group(1, resources.params_bg, &[]);
      pass.set_vertex_buffer(0, resources.pos_buf.slice(..));
      pass.draw(0..3, 0..(resources.count));
    }
    encoder.copy_texture_to_texture(
      TexelCopyTextureInfo {
        texture: self.spheres_zbuf.tex(),
        mip_level: 0,
        origin: wgpu::Origin3d { x: 0, y: 0, z: 0 },
        aspect: wgpu::TextureAspect::All,
      },
      TexelCopyTextureInfo {
        texture: self.zbuf_smoothed.tex(),
        mip_level: 0,
        origin: wgpu::Origin3d { x: 0, y: 0, z: 0 },
        aspect: wgpu::TextureAspect::All,
      },
      self.zbuf_smoothed.tex().size(),
    );
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
    self.merger.resized(device, &self.sphere_tex);
    self.zbuf_smoother.resized(device, &self.sphere_tex);
    self.merge_bg = create_merge_bg(
      device,
      &self.zbuf_smoothed,
      &self.normals,
      &self.thickness,
      &self.sphere_tex,
      &self.merge_bgl,
    );
  }

  fn render_into_pass(&self, pass: &mut wgpu::RenderPass, resources: &'a Self::RenderResources) {
    self.merger.render_into_pass(
      pass,
      &TextureDrawerResources {
        texture: &self.sphere_tex,
        bind_groups: &[&self.merge_bg, resources.global_bg],
      },
    );
  }
}

impl<'a> FluidRenderer {
  pub fn new(device: &wgpu::Device, format: &wgpu::TextureFormat, init_res: FluidRenderInit) -> Self
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
      usage: wgpu::TextureUsages::TEXTURE_BINDING
        | wgpu::TextureUsages::RENDER_ATTACHMENT
        | wgpu::TextureUsages::COPY_SRC,
      view_formats: vec![],
    };
    let spheres_zbuf = TextureProvider::new(device, desc.clone());

    desc = with!(desc: label = Some("zbuf_smoothed".to_owned()), usage = wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_DST);
    let zbuf_smoothed = TextureProvider::new(device, desc.clone());
    desc = with!(desc: label = Some("normals".to_owned()), format = *format, usage = wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::RENDER_ATTACHMENT);
    let normals = TextureProvider::new(device, desc.clone());
    desc = with!(desc: label = Some("thickness".to_owned()));
    let thickness = TextureProvider::new(device, desc.clone());
    desc = with!(desc: label = Some("sphere_tex".to_owned()));
    let sphere_tex = TextureProvider::new(device, desc);

    let module =
      device.create_shader_module(wgpu::include_wgsl!("shaders/simulation-particles.wgsl"));
    let merge_module =
      device.create_shader_module(wgpu::include_wgsl!("shaders/fluid/merger.wgsl"));
    let render_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
      label: None,
      bind_group_layouts: &[init_res.global_layout, init_res.params_layout],
      push_constant_ranges: &[],
    });
    let depth_stencil_state = DepthStencilState {
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
    };
    let tex_ty_depth = wgpu::BindingType::Texture {
      sample_type: wgpu::TextureSampleType::Depth,
      view_dimension: wgpu::TextureViewDimension::D2,
      multisampled: false,
    };
    let tex_ty_float = wgpu::BindingType::Texture {
      sample_type: wgpu::TextureSampleType::Float { filterable: true },
      view_dimension: wgpu::TextureViewDimension::D2,
      multisampled: false,
    };
    let tex_bgle = BindGroupLayoutEntry {
      binding: 0,
      visibility: ShaderStages::FRAGMENT,
      ty: tex_ty_float,
      count: None,
    };
    let merge_bgl = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
      label: Some("Merge texutures BG layout"),
      entries: &[
        with!(tex_bgle: ty = tex_ty_depth),
        with!(tex_bgle: binding = 1),
        with!(tex_bgle: binding = 2),
        with!(tex_bgle: binding = 3),
      ],
    });
    let merge_bg = create_merge_bg(
      device,
      &zbuf_smoothed,
      &normals,
      &thickness,
      &sphere_tex,
      &merge_bgl,
    );
    let sphere_render = device.create_render_pipeline(&RenderPipelineDescriptor {
      label: Some("Sphere render"),
      layout: Some(&render_layout),
      vertex: wgpu::VertexState {
        module: &module,
        entry_point: Some("vs_main"),
        compilation_options: Default::default(),
        buffers: &[PARTICLE_POS_BUFFER_LAYOUT],
      },
      primitive: wgpu::PrimitiveState {
        topology: wgpu::PrimitiveTopology::TriangleList,
        strip_index_format: None,
        front_face: wgpu::FrontFace::Ccw,
        cull_mode: None, //Some(wgpu::Face::Back),
        unclipped_depth: false,
        polygon_mode: wgpu::PolygonMode::Fill,
        conservative: false,
      },
      depth_stencil: Some(depth_stencil_state.clone()),
      multisample: MultisampleState {
        count: 1,
        mask: !0,
        alpha_to_coverage_enabled: false,
      },
      fragment: Some(wgpu::FragmentState {
        module: &module,
        entry_point: Some("fs_main"),
        compilation_options: Default::default(),
        targets: &[
          Some(wgpu::ColorTargetState {
            format: *format,
            blend: Some(wgpu::BlendState::REPLACE),
            write_mask: wgpu::ColorWrites::ALL,
          }),
          Some(wgpu::ColorTargetState {
            format: *format,
            blend: Some(wgpu::BlendState::REPLACE),
            write_mask: wgpu::ColorWrites::ALL,
          }),
        ],
      }),
      multiview: None,
      cache: None,
    });
    let zbuf_smoother = TextureDrawer::new(
      device,
      &TextureDrawerResources {
        texture: &sphere_tex,
        // TODO: add group with `spheres_zbuf` accoding to the plan
        bind_groups: &[],
      },
      format,
      TextureDrawerInitRes {
        stencil: Some(depth_stencil_state),
        fragment: None,
        layout: &[],
      },
    );
    let merger = TextureDrawer::new(
      device,
      &TextureDrawerResources {
        texture: &sphere_tex,
        bind_groups: &[&merge_bg],
      },
      format,
      TextureDrawerInitRes {
        stencil: Some(init_res.depth_stencil_state),
        fragment: Some(FragmentState {
          module: &merge_module,
          entry_point: None,
          compilation_options: Default::default(),
          targets: &[Some(wgpu::ColorTargetState {
            format: *format,
            blend: Some(wgpu::BlendState::REPLACE),
            write_mask: wgpu::ColorWrites::all(),
          })],
        }),
        layout: &[merge_bgl.clone(), init_res.global_layout.clone()],
      },
    );

    Self {
      spheres_zbuf,
      zbuf_smoothed,
      thickness,
      sphere_tex,
      normals,
      sphere_render,
      zbuf_smoother,
      merger,
      merge_bgl,
      merge_bg,
    }
  }
}

fn create_merge_bg(
  device: &wgpu::Device,
  zbuf_smoothed: &TextureProvider,
  normals: &TextureProvider,
  thickness: &TextureProvider,
  sphere_tex: &TextureProvider,
  merge_bgl: &BindGroupLayout,
) -> BindGroup {
  device.create_bind_group(&BindGroupDescriptor {
    label: Some("Merge BG"),
    layout: merge_bgl,
    entries: &[
      BindGroupEntry {
        binding: 0,
        resource: wgpu::BindingResource::TextureView(zbuf_smoothed),
      },
      BindGroupEntry {
        binding: 1,
        resource: wgpu::BindingResource::TextureView(normals),
      },
      BindGroupEntry {
        binding: 2,
        resource: wgpu::BindingResource::TextureView(sphere_tex),
      },
      BindGroupEntry {
        binding: 3,
        resource: wgpu::BindingResource::TextureView(thickness),
      },
    ],
  })
}
