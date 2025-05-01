use wgpu::{CommandEncoderDescriptor, RenderPass, TextureFormat};

pub trait ExternalResources<'a> {
  // fn update(&mut self, dt: f32, device: &wgpu::Device, queue: &wgpu::Queue) {}
  // fn bind_group_layout(&self) -> Option<&wgpu::BindGroupLayout> {
  //   None
  // }
}

impl ExternalResources<'_> for () {}

pub trait RenderTarget<'a> {
  type RenderResources: ExternalResources<'a>;
  type InitResources = ();
  type UpdateResources = Self::RenderResources;

  #[deprecated = "Implement custom `new` function for every type"]
  fn init(
    _device: &wgpu::Device,
    _queue: &wgpu::Queue,
    _resources: &'a Self::RenderResources,
    _format: &wgpu::TextureFormat,
    _init_res: Self::InitResources,
  ) -> Self
  where
    Self: Sized,
  {
    todo!()
  }
  /// Run per-frame update
  fn update(
    &mut self,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    resources: &'a Self::UpdateResources,
    encoder: &mut wgpu::CommandEncoder,
  );
  async fn update_async(
    &mut self,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    resources: &'a Self::UpdateResources,
  ) -> wgpu::CommandBuffer {
    let mut encoder = device.create_command_encoder(&CommandEncoderDescriptor { label: None });
    self.update(device, queue, resources, &mut encoder);
    encoder.finish()
  }
  /// This function is called when the render target texture is resized
  fn resized(
    &mut self,
    _device: &wgpu::Device,
    _new_size: egui::Vec2,
    _resources: &'a Self::UpdateResources,
    _format: TextureFormat,
  ) {
  }
  fn render_into_pass(&self, pass: &mut RenderPass, resources: &'a Self::RenderResources);
}
