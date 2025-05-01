use crate::render::render_target::{ExternalResources, RenderTarget};
use wgpu::{
  AddressMode, BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout,
  BindGroupLayoutEntry, DepthStencilState, Device, FilterMode, FragmentState,
  Sampler, SamplerDescriptor, ShaderStages, TextureView,
};

/// Renders the given texture into an existing render pass.
///
/// Default fragment shader just maps the pixels of the viewport to the provided texture,
/// although the behavior can be altered via specifying a custom fragment shader and depth/stencil buffer.
///
/// Note that the bind group with index `0` is occupied by the texture provided for rendering and sampler.
/// Therefore, it has the following layout:
/// ```wgsl
/// @group(0) @binding(0)
/// var tex: texture_2d<f32>;
/// @group(0) @binding(1)
/// var smp: sampler;
/// ```
/// The remaining groups can be used without limitations.
///
/// The input passed to the fragment shader is defined as follows:
/// ```wgsl
/// struct VOut {
///  @builtin(position) clip_pos: vec4f,
///  @location(0) texcoord: vec4f
/// }
/// ```
pub struct TextureDrawer {
  sampler: Sampler,
  pipeline: wgpu::RenderPipeline,
  layout: BindGroupLayout,
  bg: BindGroup,
}

pub struct TextureDrawerResources<'a> {
  pub texture: &'a TextureView,
  /// Bind groups for custom shading. The first group here has index `1`.
  pub bind_groups: &'a [&'a BindGroup],
}
impl<'a> ExternalResources<'a> for TextureDrawerResources<'a> {}

impl TextureDrawer {
  fn create_bg<'a>(
    layout: &BindGroupLayout,
    sampler: &Sampler,
    device: &Device,
    tex: &TextureView,
  ) -> BindGroup {
    let bg = device.create_bind_group(&BindGroupDescriptor {
      label: None,
      layout,
      entries: &[
        BindGroupEntry {
          binding: 0,
          resource: wgpu::BindingResource::TextureView(tex),
        },
        BindGroupEntry {
          binding: 1,
          resource: wgpu::BindingResource::Sampler(sampler),
        },
      ],
    });
    bg
  }
  pub fn resized(&mut self, device: &Device, texture: &TextureView) {
    self.bg = Self::create_bg(&self.layout, &self.sampler, device, texture);
  }
}

pub struct TextureDrawerInitRes<'a> {
  pub stencil: Option<DepthStencilState>,
  pub fragment: Option<FragmentState<'a>>,
  /// The first bind group here has `1` index
  pub layout: &'a [BindGroupLayout],
}

impl<'a> RenderTarget<'a> for TextureDrawer {
  type RenderResources = TextureDrawerResources<'a>;
  type InitResources = TextureDrawerInitRes<'a>;

  fn update(
    &mut self,
    _device: &wgpu::Device,
    _queue: &wgpu::Queue,
    _res: &Self::RenderResources,
    _encoder: &mut wgpu::CommandEncoder,
  ) {
    //nop?
  }

  fn render_into_pass(&self, pass: &mut wgpu::RenderPass, resources: &'a Self::RenderResources) {
    pass.set_pipeline(&self.pipeline);
    pass.set_bind_group(0, &self.bg, &[]);
    for (i, &g) in resources.bind_groups.iter().enumerate() {
      pass.set_bind_group((i + 1) as u32, g, &[]);
    }
    pass.draw(0..4, 0..1);
  }
}

impl TextureDrawer {
  pub fn new(
    device: &wgpu::Device,
    resources: &TextureDrawerResources,
    format: &wgpu::TextureFormat,
    mut init_res: TextureDrawerInitRes,
  ) -> Self {
    let sample_type = if format.has_depth_aspect() {
      wgpu::TextureSampleType::Depth
    } else {
      wgpu::TextureSampleType::Float { filterable: true }
    };
    let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
      label: Some("TextureDrawer layout"),
      entries: &[
        BindGroupLayoutEntry {
          binding: 0,
          visibility: ShaderStages::FRAGMENT,
          ty: wgpu::BindingType::Texture {
            sample_type,
            view_dimension: wgpu::TextureViewDimension::D2,
            multisampled: false,
          },
          count: None,
        },
        BindGroupLayoutEntry {
          binding: 1,
          visibility: ShaderStages::FRAGMENT,
          ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
          count: None,
        },
      ],
    });

    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
      label: Some("TextureDrawer"),
      bind_group_layouts: &std::iter::once(&layout)
        .chain(init_res.layout)
        .collect::<Vec<_>>(),
      push_constant_ranges: &[],
    });

    let shader = device.create_shader_module(wgpu::include_wgsl!("shaders/show-texture.wgsl"));
    let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
      label: Some("Texture render pipeline"),
      layout: Some(&pipeline_layout),
      vertex: wgpu::VertexState {
        module: &shader,
        entry_point: None,
        compilation_options: Default::default(),
        buffers: &[],
      },
      primitive: wgpu::PrimitiveState {
        topology: wgpu::PrimitiveTopology::TriangleStrip,
        strip_index_format: None,
        front_face: wgpu::FrontFace::Ccw,
        cull_mode: None,
        unclipped_depth: false,
        polygon_mode: wgpu::PolygonMode::Fill,
        conservative: false,
      },
      depth_stencil: init_res.stencil,
      multisample: wgpu::MultisampleState {
        count: 1,
        mask: !0,
        alpha_to_coverage_enabled: false,
      },
      fragment: init_res.fragment.take().or(Some(wgpu::FragmentState {
        module: &shader,
        entry_point: None,
        compilation_options: Default::default(),
        targets: &[Some(wgpu::ColorTargetState {
          format: *format,
          blend: Some(wgpu::BlendState::REPLACE),
          write_mask: wgpu::ColorWrites::all(),
        })],
      })),
      multiview: None,
      cache: None,
    });

    let sampler = device.create_sampler(&SamplerDescriptor {
      label: None,
      address_mode_u: AddressMode::ClampToEdge,
      address_mode_v: AddressMode::ClampToEdge,
      address_mode_w: AddressMode::ClampToEdge,
      mag_filter: FilterMode::Nearest,
      min_filter: FilterMode::Nearest,
      mipmap_filter: FilterMode::Nearest,
      ..Default::default() // lod_min_clamp: 0.0,
                           // lod_max_clamp: 1.0,
                           // compare: None,
                           // anisotropy_clamp: 1,
                           // border_color: None,
    });

    Self {
      pipeline,
      bg: Self::create_bg(&layout, &sampler, device, resources.texture),
      layout,
      sampler,
    }
  }
}
