use crate::render::state::*;
use eframe::CreationContext;
use egui::{Sense, Vec2};
use std::time::Instant;

pub struct App {
  time: Instant,
  startup_time: Instant,
}

impl eframe::App for App {
  fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
    let time = Instant::now();
    let dt = time - self.time;
    // self.state.update(dt.as_secs_f32(), (time - self.startup_time).as_secs_f32());
    self.time = time;
    egui::CentralPanel::default().show(ctx, |ui| {
      ui.heading("Hello World!");
      egui::Frame::canvas(ui.style()).show(ui, |ui| {
        let (rect, _) = ui.allocate_at_least(Vec2::new(800., 1200.), Sense::empty());
        ui.painter().add(egui_wgpu::Callback::new_paint_callback(
          rect,
          StateCallback {
            dt: dt.as_secs_f32(),
            time: (time - self.startup_time).as_secs_f32(),
          },
        ))
      });
    });
  }
}

impl App {
  pub fn new(cc: &CreationContext<'_>) -> Self {
    let wgpu_render_state = cc.wgpu_render_state.as_ref().unwrap();
    let state = ClearPassState::create(wgpu_render_state);
    wgpu_render_state
      .renderer
      .write()
      .callback_resources
      .insert(state);
    Self {
      time: Instant::now(),
      startup_time: Instant::now(),
    }
  }
}
