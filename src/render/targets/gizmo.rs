use core::f32;

use wgpu::{
  util::{BufferInitDescriptor, DeviceExt},
  vertex_attr_array, BufferUsages,
};

use crate::render::{
  render_target::{ExternalResources, RenderTarget},
  AsBuffer,
};

pub struct Gizmo {
  pipeline: wgpu::RenderPipeline,
  outline_pipeline: wgpu::RenderPipeline,
  vertex_buf: wgpu::Buffer,
  index_buf: wgpu::Buffer,
}

const A: f32 = 20.0;
const INNER_RADIUS: f32 = 0.5 * A * f32::consts::SQRT_3;

#[rustfmt::skip]
const AXIS_VERTICES: [f32; 12] = [
    0.0,      0.0,    INNER_RADIUS,
    0.0,   -0.5 * A, -0.5*INNER_RADIUS,
    0.0,   0.5 * A,  -0.5*INNER_RADIUS,
  200.0,     0.0,         0.0
];
#[rustfmt::skip]
const AXIS_INDICES: [u16; 6] = [
  0, 1, 2,
  3, 0, 1
];

pub struct GizmoResources<'a> {
  pub global_layout: &'a wgpu::BindGroupLayout,
  pub global_group: &'a wgpu::BindGroup,
  pub depth_stencil: &'a wgpu::DepthStencilState,
}

impl<'a> ExternalResources<'a> for GizmoResources<'a> {}

impl<'a> RenderTarget<'a> for Gizmo {
  type RenderResources = GizmoResources<'a>;

  fn init(
    device: &wgpu::Device,
    _queue: &wgpu::Queue,
    resources: &'a Self::RenderResources,
    format: &wgpu::TextureFormat,
    _: Self::InitResources,
  ) -> Self {
    let vertex_buf = device.create_buffer_init(&BufferInitDescriptor {
      label: Some("Gizmo vertex buf"),
      contents: AXIS_VERTICES.as_bytes_buffer(),
      usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
    });
    let index_buf = device.create_buffer_init(&BufferInitDescriptor {
      label: Some("Gizmo index buf"),
      contents: AXIS_INDICES.as_bytes_buffer(),
      usage: BufferUsages::INDEX,
    });

    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
      label: Some("Gizmo pipeline layout"),
      bind_group_layouts: &[resources.global_layout],
      push_constant_ranges: &[],
    });

    let shader = device.create_shader_module(wgpu::include_wgsl!("shaders/gizmo.wgsl"));
    let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
      label: Some("Gizmo render pipeline"),
      layout: Some(&pipeline_layout),
      vertex: wgpu::VertexState {
        module: &shader,
        entry_point: None,
        compilation_options: Default::default(),
        buffers: &[wgpu::VertexBufferLayout {
          array_stride: 3 * std::mem::size_of::<f32>() as u64,
          step_mode: wgpu::VertexStepMode::Vertex,
          attributes: &vertex_attr_array![0 => Float32x3],
        }],
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
      depth_stencil: Some(resources.depth_stencil.clone()),
      multisample: wgpu::MultisampleState {
        count: 1,
        mask: !0,
        alpha_to_coverage_enabled: false,
      },
      fragment: Some(wgpu::FragmentState {
        module: &shader,
        entry_point: Some("fs_main"),
        compilation_options: Default::default(),
        targets: &[Some(wgpu::ColorTargetState {
          format: *format,
          blend: Some(wgpu::BlendState::REPLACE),
          write_mask: wgpu::ColorWrites::all(),
        })],
      }),
      multiview: None,
      cache: None,
    });
    let outline_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
      label: Some("Gizmo render pipeline"),
      layout: Some(&pipeline_layout),
      vertex: wgpu::VertexState {
        module: &shader,
        entry_point: None,
        compilation_options: Default::default(),
        buffers: &[wgpu::VertexBufferLayout {
          array_stride: 3 * std::mem::size_of::<f32>() as u64,
          step_mode: wgpu::VertexStepMode::Vertex,
          attributes: &vertex_attr_array![0 => Float32x3],
        }],
      },
      primitive: wgpu::PrimitiveState {
        topology: wgpu::PrimitiveTopology::TriangleStrip,
        strip_index_format: None,
        front_face: wgpu::FrontFace::Ccw,
        cull_mode: None,
        unclipped_depth: false,
        polygon_mode: wgpu::PolygonMode::Line,
        conservative: false,
      },
      depth_stencil: Some(resources.depth_stencil.clone()),
      multisample: wgpu::MultisampleState {
        count: 1,
        mask: !0,
        alpha_to_coverage_enabled: false,
      },
      fragment: Some(wgpu::FragmentState {
        module: &shader,
        entry_point: Some("fs_outline"),
        compilation_options: Default::default(),
        targets: &[Some(wgpu::ColorTargetState {
          format: *format,
          blend: Some(wgpu::BlendState::REPLACE),
          write_mask: wgpu::ColorWrites::all(),
        })],
      }),
      multiview: None,
      cache: None,
    });

    Self {
      pipeline,
      outline_pipeline,
      vertex_buf,
      index_buf,
    }
  }

  fn render_into_pass(&self, pass: &mut wgpu::RenderPass, resources: &'a Self::RenderResources) {
    pass.set_pipeline(&self.pipeline);
    pass.set_vertex_buffer(0, self.vertex_buf.slice(..));
    pass.set_index_buffer(self.index_buf.slice(..), wgpu::IndexFormat::Uint16);
    pass.set_bind_group(0, resources.global_group, &[]);
    pass.draw_indexed(0..6, 0, 0..3);

    pass.set_pipeline(&self.outline_pipeline);
    pass.set_vertex_buffer(0, self.vertex_buf.slice(..));
    pass.set_index_buffer(self.index_buf.slice(..), wgpu::IndexFormat::Uint16);
    pass.set_bind_group(0, resources.global_group, &[]);
    pass.draw_indexed(0..6, 0, 0..3);
  }

  fn update(
    &mut self,
    _device: &wgpu::Device,
    _queue: &wgpu::Queue,
    _global: &'a Self::RenderResources,
    _encoder: &mut wgpu::CommandEncoder,
  ) {
    // nop
  }
}
