use cgmath::{ElementWise, InnerSpace, Vector2};
use core::f32;

pub trait Blur {
  #[must_use]
  fn full_kernel(&self, s: f32, side: usize, dh: Vector2<f32>) -> Vec<f32>;
  #[must_use]
  fn down_right_kernel(&self) -> Vec<f32>;
}

#[inline(always)]
pub fn scaled_len(x: usize, y: usize, dh: Vector2<f32>) -> f32 {
  dh.mul_element_wise(Vector2 {
    x: x as f32,
    y: y as f32,
  })
  .magnitude()
}

#[inline(always)]
pub fn trace_matrix(m: &Vec<Vec<f32>>) {
  for i in m {
    let mut s = String::new();
    for j in i {
      s = format!("{s} {:.5}", j);
    }
    log::trace!("{s}");
  }
}

pub struct GaussianBlur {
  pub s: f32,
  pub side: usize,
  pub dh: Vector2<f32>,
}

impl Default for GaussianBlur {
  fn default() -> Self {
    Self {
      s: 4.0,
      side: 8,
      dh: Vector2 { x: 1.0, y: 1.0 },
    }
  }
}

impl Blur for GaussianBlur {
  fn down_right_kernel(&self) -> Vec<f32> {
    let mut out = vec![vec![0.; self.side]; self.side];
    let mut norm = 0.0;
    for x in 0..self.side {
      for y in 0..self.side {
        let l = scaled_len(x, y, self.dh);
        let w =
          1. / (2. * f32::consts::PI) / self.s * self.s * f32::exp(-(l / self.s).powi(2) / 2.);
        out[x][y] = w;
        norm += w;
      }
    }
    out.into_iter().flatten().map(|x| x / norm).collect()
  }

  fn full_kernel(&self, s: f32, side: usize, dh: Vector2<f32>) -> Vec<f32> {
    let mut out = vec![vec![0.; 2 * side + 1]; 2 * side + 1];
    let cx = side;
    let cy = side;
    for x in 0..side + 1 {
      for y in 0..side + 1 {
        let l = scaled_len(x, y, dh);
        let w = 1. / (2. * f32::consts::PI) / s * s * f32::exp(-(l / s).powi(2) / 2.);
        out[cx + x][cy + y] = w;
        out[cx + x][cy - y] = w;
        out[cx - x][cy - y] = w;
        out[cx - x][cy + y] = w;
      }
    }
    if log::max_level() <= log::Level::Trace {
      log::trace!("Created blur matrix:");
      trace_matrix(&out);
    }
    out.into_iter().flatten().collect()
  }
}
