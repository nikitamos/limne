use std::sync::Arc;

use wgpu::RenderPass;

pub trait SharedResources<'a> {
  fn update(&mut self, dt: f32, device: &wgpu::Device, queue: &wgpu::Queue) {}
  fn bind_group_layout(&self) -> Option<&wgpu::BindGroupLayout> {
    None
  }
}

impl<'a> SharedResources<'a> for () {}

pub trait RenderTarget<'a> {
  type Resources<'c>: SharedResources<'a>;
  fn init<'d>(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    resources: &'d Self::Resources<'d>,
    format: &wgpu::TextureFormat,
  ) -> Self;
  fn update(
    &mut self,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    global: &wgpu::BindGroup,
    encoder: &mut wgpu::CommandEncoder,
  );
  fn render_into_pass<'b>(&self, pass: &mut RenderPass, resources: &'b Self::Resources<'b>);
}
