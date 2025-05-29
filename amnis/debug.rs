use std::time::Instant;

use egui::{PaintCallbackInfo, Pos2, Rect};
use egui_wgpu::{CallbackTrait, ScreenDescriptor, WgpuSetup};
use limne::{
  render::{
    camera::OrbitCameraController,
    state::StateCallback,
    targets::simulation::SimulationParams,
    texture_provider::{TextureProvider, TextureProviderDescriptor},
  },
  *,
};

pub mod renderdoc {
  use std::{
    os::raw::c_void,
    ptr::{self, NonNull},
  };

  #[repr(transparent)]
  pub struct Api {
    handle: ptr::NonNull<c_void>,
  }

  extern "C" {
    fn create_renderdoc_api() -> *mut c_void;
    fn destroy_renderdoc_api(api: *mut c_void);
    fn renderdoc_start_capture(api: *const c_void);
    fn renderdoc_end_capture(api: *const c_void);
  }
  impl Api {
    pub fn new() -> Option<Self> {
      unsafe {
        let handle = create_renderdoc_api();
        if handle.is_null() {
          None
        } else {
          Some(Self {
            handle: NonNull::new_unchecked(handle),
          })
        }
      }
    }
    pub fn start_frame_capture(&self) {
      unsafe {
        renderdoc_start_capture(self.handle.as_ptr());
      }
    }
    pub fn end_frame_capture(&self) {
      unsafe {
        renderdoc_end_capture(self.handle.as_ptr());
      }
    }
  }
  impl Drop for Api {
    fn drop(&mut self) {
      unsafe {
        destroy_renderdoc_api(self.handle.as_mut());
      }
    }
  }
}

#[tokio::main(flavor = "multi_thread")]
async fn main() {
  env_logger::init();
  let WgpuSetup::Existing(egui_wgpu::WgpuSetupExisting { device, queue, .. }) =
    create_wgpu_setup().await
  else {
    unreachable!()
  };
  let format = wgpu::TextureFormat::Bgra8Unorm;
  let s = render::state::PersistentState::create_raw(&device, &format, &queue);
  let mut callback_res = egui_wgpu::CallbackResources::new();
  callback_res.insert(s);

  let mut time = Instant::now();
  let begin = time;

  let mut params = SimulationParams::default();
  const SIZE: [u32; 2] = [1024, 1024];
  const SIZE_VEC: egui::Vec2 = egui::Vec2 {
    x: SIZE[0] as f32,
    y: SIZE[1] as f32,
  };
  let viewport = Rect::from_min_max(Pos2::ZERO, SIZE_VEC.to_pos2());
  let cam = OrbitCameraController::default()
    .rotate_radians(egui::Vec2 {
      x: std::f32::consts::PI / 4.,
      y: 0.0,
    })
    .get_camera();
  let target_tex = TextureProvider::new(
    &device,
    TextureProviderDescriptor {
      label: Some("Target texture".to_string()),
      size: wgpu::Extent3d {
        width: SIZE[0],
        height: SIZE[1],
        depth_or_array_layers: 1,
      },
      mip_level_count: 1,
      sample_count: 1,
      dimension: wgpu::TextureDimension::D2,
      format,
      usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
      view_formats: vec![],
    },
  );

  let api = renderdoc::Api::new().expect("failed to create Renderdoc api");
  let (tx, mut input) = tokio::sync::mpsc::channel(1);
  tokio::spawn(async move {
    loop {
      let mut input = String::new();
      std::io::stdin().read_line(&mut input).unwrap();
      tx.send(input.clone()).await.unwrap();
      if input.trim() == "die" {
        break;
      }
    }
  });
  let mut capture_count = 0;
  loop {
    if let Ok(s) = input.try_recv() {
      match s.trim().split(' ').collect::<Vec<_>>().as_slice() {
        ["rg"] => params.regen_particles = true,
        ["cap", count] => {
          capture_count = count.parse().unwrap_or(0);
        }
        ["die"] => break,
        _ => (),
      }
    }
    let t_now = Instant::now();
    let dt = (t_now - time).as_secs_f32();
    time = t_now;
    let sc = StateCallback {
      dt,
      time: (t_now - begin).as_secs_f32(),
      params: params,
      camera: cam,
      size: SIZE_VEC,
    };

    if capture_count > 0 {
      log::info!("Remaining captures: {capture_count}, fps={:.1}", 1./dt);
      api.start_frame_capture();
    }

    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
      label: Some("DebugRenderEncoder"),
    });

    let a = sc.prepare(
      &device,
      &queue,
      &ScreenDescriptor {
        size_in_pixels: SIZE,
        pixels_per_point: 1.0,
      },
      &mut encoder,
      &mut callback_res,
    );
    let b = sc.finish_prepare(&device, &queue, &mut encoder, &mut callback_res);
    let mut pass = encoder
      .begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("Final Render Pass"),
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
          view: &target_tex,
          resolve_target: None,
          ops: wgpu::Operations {
            load: wgpu::LoadOp::Clear(wgpu::Color::WHITE),
            store: wgpu::StoreOp::Store,
          },
        })],
        depth_stencil_attachment: None,
        timestamp_writes: None,
        occlusion_query_set: None,
      })
      .forget_lifetime();

    sc.paint(
      PaintCallbackInfo {
        viewport: viewport,
        clip_rect: viewport,
        pixels_per_point: 1.0,
        screen_size_px: SIZE,
      },
      &mut pass,
      &callback_res,
    );
    std::mem::drop(pass);
    queue.submit(
      a.into_iter()
        .chain(b)
        .chain(std::iter::once(encoder.finish())),
    );
    if capture_count > 0 {
      api.end_frame_capture();
      capture_count -= 1;
    }
  }
  log::info!("Exit.");
}
