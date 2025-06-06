use crate::render::state::*;
use cgmath::{num_traits::zero, InnerSpace, Vector2, Zero};
use eframe::CreationContext;
use egui::mutex::Mutex;
use egui::{Grid, Key, Rect, Sense};
use std::{f32::consts::PI, time::Instant};

use super::{
  blur::{Blur, GaussianBlur},
  camera::OrbitCameraController,
  targets::simulation::SimulationParams,
};

pub struct App {
  time_factor: f32,
  rho_from_h: bool,
  time: Instant,
  startup_time: Instant,
  viewport_rect: Rect,
  params: SimulationParams,
  controller: OrbitCameraController,
  immediate_blur: bool,
  fixed_dt: bool,
  dt: f32,
  gauss: GaussianBlur,
}

const K_RANGE: std::ops::RangeInclusive<f32> = 0.0..=1.0e10;
const M0_RANGE: std::ops::RangeInclusive<f32> = 0.0..=500.0;
const NU_RANGE: std::ops::RangeInclusive<f32> = 0.0..=1.0;

impl eframe::App for App {
  fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
    let time = Instant::now();
    let mut dt = time - self.time;
    let mut new_blur: Option<Box<dyn Blur + Send + Sync + 'static>> = None;
    self.time = time;

    egui::SidePanel::left("simulation_props").show(ctx, |ui| {
      log::trace!("left: {}", ui.available_size());
      Grid::new("sim_props_grid").show(ui, |ui| {
        ui.label("Solver");
        ui.end_row();

        ui.label("K");
        ui.add(egui::Slider::new(&mut self.params.k, K_RANGE).logarithmic(true));
        ui.end_row();

        ui.label("m0");
        ui.add(egui::Slider::new(&mut self.params.m0, M0_RANGE));
        ui.end_row();

        ui.label("ν");
        ui.add(egui::Slider::new(&mut self.params.viscosity, NU_RANGE).logarithmic(true));
        ui.end_row();

        ui.label("h");
        ui.add(egui::Slider::new(&mut self.params.h, 0.0f32..=100.0f32));
        ui.end_row();

        ui.checkbox(&mut self.rho_from_h, "Derive ρ₀");
        ui.end_row();
        if !self.rho_from_h {
          ui.label("ρ₀");
          ui.add(egui::Slider::new(&mut self.params.rho0, 20.0f32..=1500.0f32));
          ui.end_row();
        } else {
          self.params.rho0 = self.params.m0 / (4. / 3. * PI * self.params.h.powi(3));
        }

        ui.label("e");
        ui.add(egui::Slider::new(&mut self.params.e, 0.0f32..=1.0f32));
        ui.end_row();

        ui.label("w");
        ui.add(egui::Slider::new(&mut self.params.w, 0.0f32..=2.0f32));
        ui.end_row();

        ui.label("t factor");
        ui.add(egui::Slider::new(&mut self.time_factor, 0.0..=1.0));
        ui.end_row();

        ui.checkbox(&mut self.fixed_dt, "Fixed time step");
        if self.fixed_dt {
          ui.add(egui::Slider::new(&mut self.dt, 0.0..=0.7));
        } else {
          self.dt = dt.as_secs_f32();
        }
        ui.end_row();

        ui.separator();
        ui.end_row();

        ui.label("Renderer");
        ui.end_row();

        ui.label("ρ threshold");
        ui.add(egui::Slider::new(&mut self.params.dtr, 0.0f32..=10.0));
        ui.end_row();

        ui.label("T threshold");
        ui.add(egui::Slider::new(&mut self.params.ttr, 0.0f32..=10.0));
        ui.end_row();

        ui.separator();
        ui.end_row();

        ui.checkbox(&mut self.params.paused, "Paused");
        ui.end_row();

        ui.separator();
        ui.end_row();
        ui.label("Gaussian blur");
        ui.end_row();

        ui.label("σ");
        ui.add(egui::Slider::new(&mut self.gauss.s, 0.0..=15.0));
        ui.end_row();

        ui.label("Side");
        ui.add(egui::Slider::new(&mut self.gauss.side, 0..=64));
        ui.end_row();

        let apply_button = ui.button(if self.immediate_blur {
          "Un-auto-apply"
        } else {
          "Apply"
        });
        ui.end_row();
        if apply_button.secondary_clicked() {
          self.immediate_blur = !self.immediate_blur;
        }
        if self.immediate_blur || apply_button.clicked() {
          new_blur = Some(Box::new(self.gauss));
        }
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
        "V₀ = {}, m₀/ρ₀ = {}",
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
        let r = self.controller.get_radius();
        self
          .controller
          .rotate_radians(drag)
          .move_center_local(delta*r/100.)
          .move_radius(scroll.y * -0.05);

        ui.painter().add(egui_wgpu::Callback::new_paint_callback(
          rect,
          StateCallback {
            dt: self.dt * self.time_factor,
            time: (time - self.startup_time).as_secs_f32(),
            params: self.params,
            camera: self.controller.get_camera(),
            size: rect.size(),
            new_blur: Mutex::new(new_blur),
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
      rho_from_h: false,
      // Just a random rectangle
      viewport_rect: Rect::everything_above(0.0),
      params: Default::default(),
      controller: Default::default(),
      gauss: Default::default(),
      immediate_blur: false,
      fixed_dt: false, dt: 0.0
    }
  }
}
