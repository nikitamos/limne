use std::num::NonZero;

use wgpu::{
  util::{BufferInitDescriptor, DeviceExt},
  BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayoutDescriptor, BindGroupLayoutEntry,
  Buffer, BufferBinding, BufferBindingType, BufferUsages, Device, Queue,
  ShaderStages,
};
use zerocopy::{Immutable, IntoBytes};

struct SwapBuffers<T> {
  buf: [Buffer; 2],
  data: [T; 2],
  group: [BindGroup; 2],
  cur: usize,
}

pub struct SwapBuffersDescriptor {
  pub usage: BufferUsages,
  pub visibility: ShaderStages,
  ty: BufferBindingType,
  has_dynamic_offset: bool,
}

impl<T: Clone + IntoBytes + Immutable> SwapBuffers<T> {
  pub fn init_with(state: T,  dev: &Device, desc: &SwapBuffersDescriptor) -> Self {
    let bytes = state.as_bytes();
    let buf0 = dev.create_buffer_init(&BufferInitDescriptor {
      label: None,
      contents: bytes,
      usage: desc.usage,
    });
    let buf1 = dev.create_buffer_init(&BufferInitDescriptor {
      label: None,
      contents: bytes,
      usage: desc.usage,
    });

    let entry0 = BindGroupLayoutEntry {
      binding: 0,
      visibility: desc.visibility,
      ty: wgpu::BindingType::Buffer {
        ty: desc.ty,
        has_dynamic_offset: desc.has_dynamic_offset,
        min_binding_size: NonZero::new(bytes.len() as u64),
      },
      count: None,
    };
    let mut entry1 = entry0;
    entry1.binding = 1;

    let layout0 = dev.create_bind_group_layout(&BindGroupLayoutDescriptor {
      label: None,
      entries: &[entry0, entry1],
    });
    let layout2 = dev.create_bind_group_layout(&BindGroupLayoutDescriptor {
      label: None,
      entries: &[entry1, entry0],
    });

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
      layout: &layout0,
      entries: &[entry0.clone(), entry1.clone()],
    });

    let bg2 = dev.create_bind_group(&BindGroupDescriptor {
      label: None,
      layout: &layout2,
      entries: &[entry1, entry0],
    });

    Self {
      buf: [buf0, buf1],
      data: [state.clone(), state],
      group: [bg1, bg2],
      cur: 0,
    }
  }
  pub fn cur(&self) -> (&Buffer, &T) {
    (&self.buf[self.cur], &self.data[self.cur])
  }
  pub fn old(&self) -> (&Buffer, &T) {
    (&self.buf[1 - self.cur], &self.data[1 - self.cur])
  }
  pub fn cur_buf_mut(&mut self) -> &mut Buffer {
    &mut self.buf[self.cur]
  }
  pub fn cur_data_mut(&mut self) -> &mut T {
    &mut self.data[self.cur]
  }
  pub fn swap(&mut self) {
    self.cur = 1 - self.cur;
  }
  pub fn cur_size(&self) -> u64 {
    self.buf[0].size()
  }
  pub fn old_size(&self) -> u64 {
    self.buf[0].size()
  }
  pub fn write(&mut self, q: &mut Queue) {
    q.write_buffer(self.cur().0, 0, self.data[1-self.cur].as_bytes());
  }
}
