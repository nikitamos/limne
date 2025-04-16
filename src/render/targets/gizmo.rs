use wgpu::{
  util::{BufferInitDescriptor, DeviceExt},
  vertex_attr_array, Buffer, BufferUsages,
};

use crate::render::{
  render_target::{RenderTarget, SharedResources},
  AsBuffer,
};

pub struct Gizmo {
  pipeline: wgpu::RenderPipeline,
  vertex_buf: wgpu::Buffer,
}

pub const AXIS_VERTICES: [f32; 9] = [0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0];

struct GizmoResources;

impl SharedResources for GizmoResources {}

impl RenderTarget for Gizmo {
  type Resources = ();

  fn init(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    resources: &Self::Resources,
    format: &wgpu::TextureFormat,
  ) -> Self {
    let vertex_buf = device.create_buffer_init(&BufferInitDescriptor {
      label: Some("Gizmo vertex buf"),
      contents: AXIS_VERTICES.as_bytes_buffer(),
      usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
    });

    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
      label: Some("Gizmo pipeline layout"),
      bind_group_layouts: &[],
      push_constant_ranges: &[],
    });

    let shader = wgpu::include_wgsl!("shaders/gizmo.wgsl");
    let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
      label: Some("Gizmo render pipeline"),
      layout: Some(&pipeline_layout),
      vertex: wgpu::VertexState {
        module: &shader,
        entry_point: None,
        compilation_options: Default::default(),
        buffers: &[wgpu::VertexBufferLayout {
          array_stride: 3 * std::mem::size_of::<f32>(),
          step_mode: wgpu::VertexStepMode::Vertex,
          attributes: &vertex_attr_array![0 => Float32x3],
        }],
      },
      primitive: wgpu::PrimitiveState {
        topology: wgpu::PrimitiveTopology::TriangleList,
        strip_index_format: None,
        front_face: wgpu::FrontFace::Ccw,
        cull_mode: None,
        unclipped_depth: false,
        polygon_mode: wgpu::PolygonMode::Fill,
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
        entry_point: None,
        compilation_options: Default::default(),
        targets: &[Some(wgpu::ColorTargetState {
          format,
          blend: Some(wgpu::BlendState::REPLACE),
          write_mask: wgpu::ColorWrites::all(),
        })],
      }),
      multiview: None,
      cache: None,
    });

    todo!()
  }

  fn update(
    &mut self,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    global: &wgpu::BindGroup,
    encoder: &mut wgpu::CommandEncoder,
  ) {
    todo!()
  }

  fn render_into_pass(&self, pass: &mut wgpu::RenderPass, resources: &Self::Resources) {
    todo!()
  }
}
