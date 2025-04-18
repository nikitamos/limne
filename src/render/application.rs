use crate::render::state::*;
use cgmath::{num_traits::zero, InnerSpace, Point3, Vector2, Zero};
use eframe::CreationContext;
use egui::{Button, Color32, Grid, Key, Rect, Sense};
use std::time::Instant;

use super::{
  camera::OrbitCameraController,
  targets::simulation::{SimulationParams, SimulationRegenOptions},
};

pub struct App {
  time: Instant,
  startup_time: Instant,
  cell_size: String,
  v_min: String,
  v_max: String,
  viewport_rect: Rect,
  params: SimulationParams,
  controller: OrbitCameraController,
}

const K_RANGE: std::ops::RangeInclusive<f32> = 0.0..=100.0;
const M0_RANGE: std::ops::RangeInclusive<f32> = 0.0..=100.0;
const NU_RANGE: std::ops::RangeInclusive<f32> = 0.0..=10.0;

impl eframe::App for App {
  fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
    let time = Instant::now();
    let dt = time - self.time;
    // self.state.update(dt.as_secs_f32(), (time - self.startup_time).as_secs_f32());
    self.time = time;
    let mut regen_opts = None;
    let mut regen_pos = false;

    egui::SidePanel::left("simulation_props").show(ctx, |ui| {
      Grid::new("sim_props_grid").show(ui, |ui| {
        ui.label("K");
        ui.add(egui::Slider::new(&mut self.params.k, K_RANGE).logarithmic(true));
        ui.end_row();
        ui.label("m0");
        ui.add(egui::Slider::new(&mut self.params.m0, M0_RANGE).logarithmic(true));
        ui.end_row();
        ui.label("ν");
        ui.add(egui::Slider::new(&mut self.params.viscosity, NU_RANGE));
        ui.end_row();

        ui.checkbox(&mut self.params.paused, "Paused");
        ui.end_row();
        if !self.params.paused {
          ui.checkbox(&mut self.params.move_particles, "Move particles");
          ui.end_row();
        }
        ui.separator();
        ui.end_row();
        ui.checkbox(&mut self.params.draw_density_field, "Draw density field");
        ui.end_row();
        ui.checkbox(&mut self.params.draw_particles, "Draw particles");
        ui.end_row();
      });
      ui.collapsing("Re-generate grid", |ui| {
        Grid::new("regen_grid_opts").show(ui, |ui| {
          ui.label("Cell size");
          ui.text_edit_singleline(&mut self.cell_size);
          ui.end_row();

          ui.label("Min speed");
          ui.text_edit_singleline(&mut self.v_min);
          ui.end_row();

          ui.label("Max speed");
          ui.text_edit_singleline(&mut self.v_max);
          ui.end_row();

          if let Some((size, vmin, vmax)) = self
            .cell_size
            .parse::<f32>()
            .ok()
            .zip(self.v_min.parse::<f32>().ok())
            .zip_with(self.v_max.parse::<f32>().ok(), |(x, y), z| (x, y, z))
          {
            if ui.add_enabled(true, Button::new("Do it!")).clicked() {
              regen_opts = Some(SimulationRegenOptions { size, vmin, vmax });
            }
          } else {
            ui.colored_label(Color32::DARK_RED, "Invalid input");
          }
        })
      });
      regen_pos = ui.button("Regen positions").clicked();

      ui.label(format!(
        "Viewport size: {}x{}",
        self.viewport_rect.width() as usize,
        self.viewport_rect.height() as usize
      ));
      ui.end_row();
      let camera_pos = self.controller.get_pos();
      ui.label(format!(
        "Frame time: {:.2}ms, {:.0} FPS\n Camera at: ({:.1} {:.1} {:.1})`",
        dt.as_millis_f32(),
        1.0 / dt.as_secs_f32(),
        camera_pos.x,
        camera_pos.y,
        camera_pos.z
      ));
      if ui.button("Reset camera").clicked() {
        self.controller.reset();
      }
    });
    egui::CentralPanel::default().show(ctx, |ui| {
      egui::Frame::canvas(ui.style()).show(ui, |ui| {
        let (rect, resp) = ui.allocate_exact_size(ui.available_size(), Sense::all());
        let drag = resp.drag_motion() * 0.07;
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
          .move_radius(scroll.y * -0.3);
        // .look_at(Point3::new(0.0, 0.0, 0.0)) //-rect.width() / 2., -rect.height() / 2., 0.))

        ui.painter().add(egui_wgpu::Callback::new_paint_callback(
          rect,
          StateCallback {
            dt: dt.as_secs_f32(),
            time: (time - self.startup_time).as_secs_f32(),
            regen_opts,
            regen_pos,
            params: self.params,
            camera: self.controller.get_camera(),
          },
        ));
        self.viewport_rect = rect;
      });
    });
    ctx.request_repaint();
  }
}

impl App {
  pub fn new(cc: &CreationContext<'_>) -> Self {
    let wgpu_render_state = cc.wgpu_render_state.as_ref().unwrap();
    let opts = SimulationRegenOptions::default();
    let state = PersistentState::create(wgpu_render_state, opts);
    wgpu_render_state
      .renderer
      .write()
      .callback_resources
      .insert(state);
    Self {
      time: Instant::now(),
      startup_time: Instant::now(),
      cell_size: opts.size.to_string(),
      v_max: opts.vmax.to_string(),
      v_min: opts.vmin.to_string(),
      // Just a random rectangle
      viewport_rect: Rect::everything_above(0.0),
      params: Default::default(),
      controller: Default::default(),
    }
  }
}
