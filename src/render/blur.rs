use cgmath::{ElementWise, InnerSpace, Vector2};
use core::f32;

pub trait Blur {
  #[must_use]
  fn full_kernel(&self) -> Vec<f32>;
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
pub fn trace_matrix(m: impl Iterator<Item = impl Iterator<Item = f32>>) {
  let mut sum = 0.0;
  for i in m {
    let mut s = String::new();
    for j in i {
      sum += j;
      s = format!("{s} {j:.5}");
    }
    log::trace!("{s}");
  }
  log::trace!("Matrix sum={sum:.5}");
}

#[derive(Copy, Clone)]
pub struct GaussianBlur {
  pub s: f32,
  pub side: usize,
  pub dh: Vector2<f32>,
}

impl Default for GaussianBlur {
  fn default() -> Self {
    Self {
      s: 7.0,
      side: 12,
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
        let w = 1. / f32::consts::TAU / self.s * self.s * f32::exp(-(l / self.s).powi(2) / 2.);
        out[x][y] = w;
        norm += w;
      }
    }
    out.into_iter().flatten().map(|x| x / norm).collect()
  }

  fn full_kernel(&self) -> Vec<f32> {
    let side = self.side;
    let mut out = vec![vec![0.; 2 * side + 1]; 2 * side + 1];
    let cx = side;
    let cy = side;
    let mut norm = 0.;
    for x in 0..side + 1 {
      for y in 0..side + 1 {
        let l = scaled_len(x, y, self.dh);
        let w =
          1. / (2. * f32::consts::PI) / self.s * self.s * f32::exp(-(l / self.s).powi(2) / 2.);
        out[cx + x][cy + y] = w;
        out[cx + x][cy - y] = w;
        out[cx - x][cy - y] = w;
        out[cx - x][cy + y] = w;
        if x != 0 && y != 0 {
          norm += 4. * w;
        } else if (x == 0) ^ (y == 0) {
          norm += 2. * w;
        } else {
          norm += w;
        }
      }
    }
    if log::max_level() >= log::Level::Trace {
      log::trace!("Created blur matrix:");
      trace_matrix(out.iter().map(|x| x.iter().map(|y| y / norm)));
    }
    out.into_iter().flatten().map(|x| x / norm).collect()
  }
}
