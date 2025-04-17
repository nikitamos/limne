use wgpu::RenderPass;

pub trait ExternalResources<'a> {
  // fn update(&mut self, dt: f32, device: &wgpu::Device, queue: &wgpu::Queue) {}
  // fn bind_group_layout(&self) -> Option<&wgpu::BindGroupLayout> {
  //   None
  // }
}

impl<'a> ExternalResources<'a> for () {}

pub trait RenderTarget<'a> {
  type Resources: ExternalResources<'a>;
  type InitResources = ();

  fn init(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    resources: &'a Self::Resources,
    format: &wgpu::TextureFormat,
    init_res: Self::InitResources,
  ) -> Self;
  fn update(
    &mut self,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    resources: &'a Self::Resources,
    encoder: &mut wgpu::CommandEncoder,
  );
  fn resized(
    &mut self,
    _device: &wgpu::Device,
    _new_size: egui::Vec2,
    _resources: &'a Self::Resources,
  ) {
  }
  fn render_into_pass(&self, pass: &mut RenderPass, resources: &'a Self::Resources);
}
