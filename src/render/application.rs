use crate::render::state::*;
use eframe::CreationContext;
use egui::{Button, Color32, Grid, Rect, Sense};
use std::time::Instant;

use super::simulation::SimulationRegenOptions;

pub struct App {
  time: Instant,
  startup_time: Instant,
  cell_size: String,
  v_min: String,
  v_max: String,
  k: f64,
  viewport_rect: Rect,
}

const K_RANGE: std::ops::RangeInclusive<f64> = 0.0..=1000.0;

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
        ui.label("Constant K");
        ui.add(egui::Slider::new(&mut self.k, K_RANGE));
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
      ui.label(format!("Frame time: {:.2}ms, {:.0} FPS", dt.as_millis_f32(), 1.0 / dt.as_secs_f32()));
    });
    egui::CentralPanel::default().show(ctx, |ui| {
      egui::Frame::canvas(ui.style()).show(ui, |ui| {
        let (rect, _) = ui.allocate_exact_size(ui.available_size(), Sense::empty());
        ui.painter().add(egui_wgpu::Callback::new_paint_callback(
          rect,
          StateCallback {
            dt: dt.as_secs_f32(),
            time: (time - self.startup_time).as_secs_f32(),
            regen_opts,
            regen_pos
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
    let state = PersistentState::create(wgpu_render_state);
    wgpu_render_state
      .renderer
      .write()
      .callback_resources
      .insert(state);
    Self {
      time: Instant::now(),
      startup_time: Instant::now(),
      cell_size: String::new(),
      v_max: String::new(),
      v_min: String::new(),
      k: 42.0,
      // Just a random rectangle
      viewport_rect: Rect::everything_above(0.0),
    }
  }
}
