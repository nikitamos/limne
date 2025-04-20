use rayon::prelude::*;
use std::ops::{Add, Deref, DerefMut, Mul};
use std::slice;
use std::sync::Arc;

use cgmath::{num_traits, EuclideanSpace, InnerSpace, Point3, Vector3};

use crate::render::AsBuffer;

pub mod kernels {
  use core::f32;
  const fn cubed(x: f32) -> f32 {
    x * x * x
  }
  pub const fn poly6(r: f32, h: f32) -> f32 {
    if 0. <= r && r <= h {
      315. / 64. / f32::consts::PI / h * cubed(h * h - r * r)
    } else {
      0.
    }
  }
  pub const fn spiky(r: f32, h: f32) -> f32 {
    if 0. <= r && r <= h {
      15. / f32::consts::PI / cubed(h * h) * cubed(h - r)
    } else {
      0.
    }
  }
}

#[derive(Clone, Debug)]
#[repr(C)]
pub struct Particle {
  pub pos: Point3<f32>,
  pub density: f32,
  pub velocity: Vector3<f32>,
}

impl AsBuffer for &[Particle] {
  fn as_bytes_buffer(&self) -> &[u8] {
    unsafe {
      slice::from_raw_parts(
        self.as_ptr().cast(),
        std::mem::size_of::<Particle>() * self.len(),
      )
    }
  }
}

pub struct Solver {
  m0: f32,
  rho0: f32,
  particles: Vec<Particle>,
  old_particles: Vec<Particle>,
}

impl Solver {
  pub fn new(m0: f32, rho_0: f32, particles: Vec<Particle>) -> Self {
    Self {
      m0,
      rho0: rho_0,
      old_particles: particles.clone(),
      particles,
    }
  }
  pub fn particles(&self) -> &[Particle] {
    &self.particles
  }
  pub fn particles_mut(&mut self) -> &mut [Particle] {
    &mut self.particles
  }
  pub fn reset(&mut self, v: Vec<Particle>) {
    self.particles = v.clone();
    self.old_particles = v;
  }

  fn interp_at<U, Kernel, Field>(&self, kern: Kernel, location: Point3<f32>, field: Field) -> U
  where
    U: Send + Sync + Add<U> + std::iter::Sum<U> + Mul<f32, Output = U> + num_traits::Zero,
    Kernel: Fn(f32, f32) -> f32 + Send + Sync,
    Field: Fn(&Particle) -> U + Send + Sync,
  {
    let kern = Arc::new(kern);
    self
      .old_particles
      .par_iter()
      .map(|x| {
        let r = (x.pos - location).magnitude();
        field(x) * (kern(r, 1.0) / x.density)
      })
      .sum()
  }

  pub fn update(&mut self, dt: f32, k: f32, m0: f32, rho0: f32) {
    self.m0 = m0;
    self.rho0 = rho0;
    std::mem::swap(&mut self.particles, &mut self.old_particles);
    // Solve
    // 1. Density

    self.particles.iter_mut().for_each(|p| {
    let a = m0 * self.interp_at(kernels::poly6, Point3::origin(), |x| x.density);
    let rho = todo!();
    // 2. Pressure
    let p = k * (rho - rho0);

    // 2. Viscosity (temporarily 0)
    // 3. Surface tension
    // 4. Integrate forces

    // Move
      x.pos += dt * p.velocity;
    });
  }
}

impl Deref for Solver {
  type Target = [Particle];

  fn deref(&self) -> &Self::Target {
    &self.particles
  }
}

impl DerefMut for Solver {
  fn deref_mut(&mut self) -> &mut Self::Target {
    &mut self.particles
  }
}
