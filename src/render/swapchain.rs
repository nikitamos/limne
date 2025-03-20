use std::num::NonZero;
use wgpu::{
  util::{BufferInitDescriptor, DeviceExt},
  BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor,
  BindGroupLayoutEntry, Buffer, BufferBinding, BufferBindingType, BufferUsages, CommandEncoder,
  Device, Queue, ShaderStages,
};

use super::simulation::AsBuffer;

pub struct SwapBuffers<T> {
  buf: [Buffer; 2],
  data: [T; 2],
  group: [BindGroup; 2],
  cur: usize,
  layout: BindGroupLayout,
  desc: SwapBuffersDescriptor
}

pub struct SwapBuffersDescriptor {
  pub usage: BufferUsages,
  pub visibility: ShaderStages,
  pub ty: BufferBindingType,
  pub has_dynamic_offset: bool,
}

impl<T: Clone + AsBuffer> SwapBuffers<T> {
  pub fn init_with(state: T, dev: &Device, desc: SwapBuffersDescriptor) -> Self {
    let layout = Self::create_bind_group_layout(dev, &desc);
    let (buf0, buf1, bg1, bg2) = Self::create_binding_groups(&state, dev, &desc, &layout);
    Self {
      buf: [buf0, buf1],
      data: [state.clone(), state],
      group: [bg1, bg2],
      cur: 0,
      layout,
      desc,
    }
  }
  pub fn cur(&self) -> &T {
    &self.data[self.cur]
  }
  pub fn old(&self) -> (&Buffer, &T) {
    (&self.buf[1 - self.cur], &self.data[1 - self.cur])
  }
  pub fn cur_buf(&self) -> &Buffer {
    &self.buf[self.cur]
  }
  pub fn swap(&mut self, encoder: &mut CommandEncoder) {
    encoder.copy_buffer_to_buffer(self.cur_buf(), 0, self.old().0, 0, self.cur_size());
    self.cur = 1 - self.cur;
  }
  pub fn cur_size(&self) -> u64 {
    self.buf[self.cur].size()
  }
  pub fn old_size(&self) -> u64 {
    self.buf[1 - self.cur].size()
  }
  pub fn write(&mut self, q: &mut Queue) {
    q.write_buffer(self.cur_buf(), 0, self.data[self.cur].as_bytes_buffer());
  }
  pub fn cur_group(&self) -> &BindGroup {
    &self.group[self.cur]
  }
  pub fn cur_layout(&self) -> &BindGroupLayout {
    &self.layout
  }
  pub fn reset(&mut self, new: T, device: &Device) {
    let (buf0, buf1, bg0, bg1) =
      Self::create_binding_groups(&new, device, &self.desc, &self.layout);
    self.buf[0] = buf0;
    self.buf[1] = buf1;
    self.group[0] = bg0;
    self.group[1] = bg1;
    self.data[0] = new.clone();
    self.data[1] = new;
  }

  fn create_bind_group_layout(dev: &Device, desc: &SwapBuffersDescriptor) -> BindGroupLayout {
    let entry0 = BindGroupLayoutEntry {
      binding: 0,
      visibility: desc.visibility,
      ty: wgpu::BindingType::Buffer {
        ty: desc.ty,
        has_dynamic_offset: desc.has_dynamic_offset,
        min_binding_size: None, //NonZero::new(bytes.len() as u64),
      },
      count: None,
    };
    let mut entry1 = entry0;
    entry1.binding = 1;

    dev.create_bind_group_layout(&BindGroupLayoutDescriptor {
      label: None,
      entries: &[entry0, entry1],
    })
  }

  fn create_binding_groups(
    state: &T,
    dev: &Device,
    desc: &SwapBuffersDescriptor,
    layout: &BindGroupLayout,
  ) -> (Buffer, Buffer, BindGroup, BindGroup) {
    let bytes = state.as_bytes_buffer();
    // "new"
    let buf0 = dev.create_buffer_init(&BufferInitDescriptor {
      label: None,
      contents: bytes,
      usage: desc.usage,
    });
    // "old"
    let buf1 = dev.create_buffer_init(&BufferInitDescriptor {
      label: None,
      contents: bytes,
      usage: desc.usage,
    });

    // Create the BGs
    let entry0 = BindGroupEntry {
      binding: 0,
      resource: wgpu::BindingResource::Buffer(BufferBinding {
        buffer: &buf0,
        offset: 0,
        size: None,
      }),
    };
    let entry1 = BindGroupEntry {
      binding: 1,
      resource: wgpu::BindingResource::Buffer(BufferBinding {
        buffer: &buf1,
        offset: 0,
        size: None,
      }),
    };

    let bg1 = dev.create_bind_group(&BindGroupDescriptor {
      label: None,
      layout: &layout,
      entries: &[entry0.clone(), entry1.clone()],
    });

    let bg2 = dev.create_bind_group(&BindGroupDescriptor {
      label: None,
      layout: &layout,
      entries: &[entry1, entry0],
    });

    (buf0, buf1, bg1, bg2)
  }
}
