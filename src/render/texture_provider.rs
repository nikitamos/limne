use std::ops::Deref;

use wgpu::{
  Device, Extent3d, Texture, TextureDescriptor, TextureDimension, TextureFormat, TextureUsages,
  TextureView, TextureViewDescriptor,
};

#[derive(Clone)]
pub struct TextureProviderDescriptor {
  pub label: Option<String>,
  pub size: Extent3d,
  pub mip_level_count: u32,
  pub sample_count: u32,
  pub dimension: TextureDimension,
  pub format: TextureFormat,
  pub usage: TextureUsages,
  pub view_formats: Vec<TextureFormat>,
}

impl TextureProviderDescriptor {
  pub fn tex_descriptor<'a>(&'a self) -> TextureDescriptor<'a> {
    TextureDescriptor {
      label: self.label.as_deref(),
      size: self.size,
      mip_level_count: self.mip_level_count,
      sample_count: self.sample_count,
      dimension: self.dimension,
      format: self.format,
      usage: self.usage,
      view_formats: &self.view_formats,
    }
  }
}

pub struct TextureProvider {
  tex: Texture,
  view: TextureView,
  desc: TextureProviderDescriptor,
}

impl TextureProvider {
  pub fn new<'a>(device: &Device, desc: TextureProviderDescriptor) -> Self {
    let (tex, view) = Self::create_tex_view(device, &desc.tex_descriptor());
    Self { tex, view, desc }
  }

  fn create_tex_view(device: &Device, desc: &TextureDescriptor) -> (Texture, TextureView) {
    log::trace!(
      "Creating TextureProvider '{}' w/format {:?}",
      desc.label.unwrap_or_default(),
      desc.format
    );
    let view_label = desc.label.map(|s| {
      let mut p = "View of ".to_owned();
      p.push_str(&s);
      p
    });
    let tex = device.create_texture(desc);
    let view_descriptor = TextureViewDescriptor {
      label: view_label.as_deref(),
      format: Some(desc.format),
      dimension: None,
      usage: None,
      aspect: wgpu::TextureAspect::All,
      base_mip_level: 0, // 1?
      mip_level_count: None,
      base_array_layer: 0,
      array_layer_count: None,
    };
    let view = tex.create_view(&view_descriptor);
    (tex, view)
  }

  pub fn tex(&self) -> &Texture {
    &self.tex
  }
  pub fn view(&self) -> &TextureView {
    &self.view
  }
  pub fn resize(&mut self, device: &Device, size: Extent3d) {
    self.desc.size = size;
    (self.tex, self.view) = Self::create_tex_view(device, &self.desc.tex_descriptor());
  }
  pub fn format(&self) -> TextureFormat {
    self.tex.format()
  }
}

impl Deref for TextureProvider {
  type Target = TextureView;

  fn deref(&self) -> &Self::Target {
    self.view()
  }
}
