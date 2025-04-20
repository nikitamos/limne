use std::ops::{Deref, DerefMut};

use wgpu::{
  util::{BufferInitDescriptor, DeviceExt},
  BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor,
  BindGroupLayoutEntry, Buffer, BufferBinding, BufferBindingType, BufferDescriptor, BufferSize,
  BufferUsages, CommandEncoder, Device, Queue, ShaderStages,
};

use super::AsBuffer;

struct SwapBuffer<T> {
  buf: Buffer,
  data: T,
  group: BindGroup,
}

pub struct SwapBuffers<T> {
  cur: SwapBuffer<T>,
  old: SwapBuffer<T>,
  layout: BindGroupLayout,
  desc: SwapBuffersDescriptor,
}

mod buf_guard {
  use super::*;
  pub struct BufGuard<'b, T>
  where
    T: Clone + AsBuffer,
  {
    bufs: &'b mut SwapBuffers<T>,
    encoder: &'b mut CommandEncoder,
    queue: &'b Queue,
  }

  impl<'b, T> BufGuard<'b, T>
  where
    T: Clone + AsBuffer,
  {
    pub fn new(
      bufs: &'b mut SwapBuffers<T>,
      encoder: &'b mut CommandEncoder,
      queue: &'b Queue,
    ) -> Self {
      bufs.fetch(queue);
      Self {
        bufs,
        encoder,
        queue,
      }
    }
  }

  impl<T> Deref for BufGuard<'_, T>
  where
    T: Clone + AsBuffer,
  {
    type Target = T;

    fn deref(&self) -> &Self::Target {
      &self.bufs.cur.data
    }
  }

  impl<T> DerefMut for BufGuard<'_, T>
  where
    T: Clone + AsBuffer,
  {
    fn deref_mut(&mut self) -> &mut Self::Target {
      &mut self.bufs.cur.data
    }
  }

  impl<T: Clone + AsBuffer> Drop for BufGuard<'_, T> {
    fn drop(&mut self) {
      todo!()
    }
  }
}

pub use buf_guard::BufGuard;

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
      cur: SwapBuffer {
        buf: buf0,
        data: state.clone(),
        group: bg1,
      },
      old: SwapBuffer {
        buf: buf1,
        data: state,
        group: bg2,
      },
      layout,
      desc,
    }
  }
  pub fn cur(&self) -> &T {
    &self.cur.data
  }
  pub fn old(&self) -> (&Buffer, &T) {
    (&self.old.buf, &self.old.data)
  }
  pub fn cur_buf(&self) -> &Buffer {
    &self.cur.buf
  }
  pub fn swap(&mut self, encoder: &mut CommandEncoder) {
    encoder.copy_buffer_to_buffer(self.cur_buf(), 0, self.old().0, 0, self.cur_buf().size());
    std::mem::swap(&mut self.cur, &mut self.old);
  }
  #[must_use]
  pub fn update<'a>(
    &'a mut self,
    queue: &'a Queue,
    encoder: &'a mut CommandEncoder,
  ) -> BufGuard<'a, T> {
    BufGuard::new(self, encoder, queue)
  }

  /// Fetches the current buffer from GPU.
  /// The old buffer remains unchanged since it is considered immutable.
  fn fetch(&mut self, q: &Queue) {
    self
      .cur_buf()
      .slice(..)
      .map_async(wgpu::MapMode::Read, |r| {
        dbg!(r);
      });
    println!("Before submit");
    q.submit([]);
    println!("After submit");
    let mr = self.cur_buf().slice(..).get_mapped_range();
    self.cur_buf().unmap();
  }
  /// Sends current buffer to the GPU.
  fn send_cur(&self) {
    todo!()
  }
  fn swap_cur_to_old() {
    todo!()
  }

  pub fn map_cpu_only<'r, U, F: FnOnce(&mut T, &T) -> U>(&'r mut self, map: F) -> U {
    map(&mut self.cur.data, &self.old.data)
  }

  pub fn cur_group(&self) -> &BindGroup {
    &self.cur.group
  }

  pub fn layout(&self) -> &BindGroupLayout {
    &self.layout
  }
  #[deprecated(note = "This does not work properly. Consider using [`Self::update`] instead")]
  pub fn reset(&mut self, new: T, device: &Device) {
    let (buf0, buf1, bg0, bg1) =
      Self::create_binding_groups(&new, device, &self.desc, &self.layout);

    self.cur = SwapBuffer {
      buf: buf0,
      data: new.clone(),
      group: bg0,
    };
    self.old = SwapBuffer {
      buf: buf1,
      data: new,
      group: bg1,
    };
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
    let buf0 = dev.create_buffer(&BufferDescriptor {
      label: Some("new buf"),
      size: bytes.len() as u64,
      usage: desc.usage,
      mapped_at_creation: true,
    });
    buf0.slice(..).get_mapped_range_mut().copy_from_slice(bytes);
    buf0.unmap();
    // "old"
    let buf1 = dev.create_buffer_init(&BufferInitDescriptor {
      label: Some("old buf"),
      contents: bytes,
      usage: BufferUsages::STORAGE | BufferUsages::MAP_READ | BufferUsages::COPY_DST,
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
      layout,
      entries: &[entry0.clone(), entry1.clone()],
    });

    let bg2 = dev.create_bind_group(&BindGroupDescriptor {
      label: None,
      layout,
      entries: &[entry1, entry0],
    });

    (buf0, buf1, bg1, bg2)
  }
}
