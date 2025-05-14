use crate::render::state::*;
use cgmath::{num_traits::zero, InnerSpace, Vector2, Zero};
use eframe::CreationContext;
use egui::{Grid, Key, Rect, Sense};
use std::{f32::consts::PI, time::Instant};

use super::{camera::OrbitCameraController, targets::simulation::SimulationParams};

pub struct App {
  time_factor: f32,
  time: Instant,
  startup_time: Instant,
  viewport_rect: Rect,
  params: SimulationParams,
  controller: OrbitCameraController,
}

const K_RANGE: std::ops::RangeInclusive<f32> = 0.0..=500.0;
const M0_RANGE: std::ops::RangeInclusive<f32> = 0.0..=500.0;
const NU_RANGE: std::ops::RangeInclusive<f32> = 0.0..=10.0;

impl eframe::App for App {
  fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
    let time = Instant::now();
    let dt = time - self.time;
    self.time = time;

    egui::SidePanel::left("simulation_props").show(ctx, |ui| {
      log::trace!("left: {}", ui.available_size());
      Grid::new("sim_props_grid").show(ui, |ui| {
        ui.label("K");
        ui.add(egui::Slider::new(&mut self.params.k, K_RANGE));
        ui.end_row();
        ui.label("m0");
        ui.add(egui::Slider::new(&mut self.params.m0, M0_RANGE));
        ui.end_row();
        ui.label("ν");
        ui.add(egui::Slider::new(&mut self.params.viscosity, NU_RANGE));
        ui.end_row();

        ui.label("h");
        ui.add(egui::Slider::new(&mut self.params.h, 0.0f32..=100.0f32));
        ui.end_row();

        ui.label("ρ₀");
        ui.add(egui::Slider::new(&mut self.params.rho0, 0.0f32..=20.0f32));
        ui.end_row();

        ui.label("e");
        ui.add(egui::Slider::new(&mut self.params.e, 0.0f32..=1.0f32));
        ui.end_row();

        ui.label("w");
        ui.add(egui::Slider::new(&mut self.params.w, 0.0f32..=500.0f32));
        ui.end_row();

        ui.label("t factor");
        ui.add(egui::Slider::new(&mut self.time_factor, 0.0..=1.0));
        ui.end_row();

        ui.checkbox(&mut self.params.paused, "Paused");
        ui.end_row();
        if !self.params.paused {
          ui.checkbox(&mut self.params.move_particles, "Move particles");
          ui.end_row();
        }
        ui.separator();
        ui.end_row();
        ui.checkbox(&mut self.params.draw_particles, "Draw particles");
        ui.end_row();
      });
      self.params.regen_particles = ui.button("Regen positions").clicked();

      ui.label(format!(
        "Viewport size: {}x{}",
        self.viewport_rect.width() as usize,
        self.viewport_rect.height() as usize
      ));
      ui.end_row();
      let camera_pos = self.controller.get_pos();
      let camera_center = self.controller.get_center();
      ui.label(format!(
        "Frame time: {:.2}ms, {:.0} FPS\nCamera at: ({:.1} {:.1} {:.1})
Looks at: ({:.1}, {:.1}, {:.1})\nr={:.1}",
        dt.as_millis_f32(),
        1.0 / dt.as_secs_f32(),
        camera_pos.x,
        camera_pos.y,
        camera_pos.z,
        camera_center.x,
        camera_center.y,
        camera_center.z,
        self.controller.get_radius()
      ));
      if ui.button("Reset camera").clicked() {
        self.controller.reset();
      }
      ui.label(format!(
        "V_0 = {}, m_0/rho_0 = {}",
        4.0 / 3.0 * PI * self.params.h.powi(3),
        self.params.m0 / self.params.rho0
      ));
    });
    egui::CentralPanel::default().show(ctx, |ui| {
      egui::Frame::canvas(ui.style()).show(ui, |ui| {
        let (rect, resp) = ui.allocate_exact_size(ui.available_size(), Sense::all());
        let drag = resp.drag_motion() * 0.02;
        let (fwd, back, right, left, scroll) = ui.input(|i| {
          (
            i.key_down(Key::W),
            i.key_down(Key::S),
            i.key_down(Key::D),
            i.key_down(Key::A),
            i.smooth_scroll_delta,
          )
        });
        let mut delta = Vector2::<_>::zero();
        if fwd {
          delta.x += 1.0;
        }
        if back {
          delta.x -= 1.0;
        }
        if right {
          delta.y += 1.0;
        }
        if left {
          delta.y -= 1.0;
        }
        if delta != zero() {
          delta = delta.normalize() * 2.0;
        }
        self
          .controller
          .handle_drag(drag)
          .move_center_local(delta)
          .move_radius(scroll.y * -0.5);

        ui.painter().add(egui_wgpu::Callback::new_paint_callback(
          rect,
          StateCallback {
            dt: dt.as_secs_f32() * self.time_factor,
            time: (time - self.startup_time).as_secs_f32(),
            params: self.params,
            camera: self.controller.get_camera(),
            size: rect.size()
          },
        ));
        self.viewport_rect = rect;
      });
    });
    ctx.request_repaint();
  }
  fn save(&mut self, _storage: &mut dyn eframe::Storage) {
    eprintln!("SAVE!");
  }
}

impl App {
  pub fn new(cc: &CreationContext<'_>) -> Self {
    let wgpu_render_state = cc.wgpu_render_state.as_ref().unwrap();
    let state = PersistentState::create_egui(wgpu_render_state);
    wgpu_render_state
      .renderer
      .write()
      .callback_resources
      .insert(state);
    Self {
      time_factor: 1.0,
      time: Instant::now(),
      startup_time: Instant::now(),
      // Just a random rectangle
      viewport_rect: Rect::everything_above(0.0),
      params: Default::default(),
      controller: Default::default(),
    }
  }
}
