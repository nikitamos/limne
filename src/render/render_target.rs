use std::sync::Arc;

use wgpu::RenderPass;

pub trait SharedResources {
  fn update(&mut self, dt: f32, device: &wgpu::Device, queue: &wgpu::Queue) {}
  fn bind_group_layout(&self) -> Option<&wgpu::BindGroupLayout> {
    None
  }
}

impl SharedResources for () {}

pub trait RenderTarget {
  type Resources: SharedResources;
  fn init(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    resources: &Self::Resources,
    format: &wgpu::TextureFormat,
  ) -> Self;
  fn update(
    &mut self,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    global: &wgpu::BindGroup,
    encoder: &mut wgpu::CommandEncoder,
  );
  fn render_into_pass(&self, pass: &mut RenderPass, resources: &Self::Resources);
}
