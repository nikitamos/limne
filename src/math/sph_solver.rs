use kernels::grad_spiky;
use rayon::prelude::*;
use std::ops::{Add, Deref, DerefMut, Mul};
use std::slice;

use cgmath::{num_traits, EuclideanSpace, InnerSpace, Point3, Vector3, Zero};

use crate::render::AsBuffer;

pub mod kernels {
  use core::f32;

  use cgmath::{InnerSpace, Vector3, Zero};
  fn cubed(x: f32) -> f32 {
    x.powi(3)
  }
  pub fn poly6(r: f32, h: f32) -> f32 {
    if 0. <= r && r <= h {
      315. / 64. / f32::consts::PI / h.powi(9) * cubed(h * h - r * r)
    } else {
      0.
    }
  }
  pub fn spiky(r: f32, h: f32) -> f32 {
    if 0. <= r && r <= h {
      15. / f32::consts::PI / cubed(h * h) * cubed(h - r)
    } else {
      0.
    }
  }
  pub fn grad_spiky(r: Vector3<f32>, h: f32) -> Vector3<f32> {
    if r.magnitude() >= h || r.magnitude() == 0. {
      Vector3::zero()
    } else {
      -45. * (h - r.magnitude()).powi(2) / f32::consts::PI / h.powi(6) * r.normalize()
    }
  }
}

#[derive(Clone, Debug)]
pub struct Particle {
  pub pos: Point3<f32>,
  pub density: f32,
  pub velocity: Vector3<f32>,
  _padding1: u32,
  forces: Vector3<f32>,
  _padding2: u32,
}

impl Default for Particle {
  fn default() -> Self {
    Self {
      forces: Vector3::zero(),
      pos: Point3::origin(),
      density: 1.0,
      _padding1: 0,
      velocity: Vector3::zero(),
      _padding2: 0
    }
  }
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
  h: f32,
  m0: f32,
  rho0: f32,
  particles: Vec<Particle>,
  old_particles: Vec<Particle>,
}

impl Solver {
  pub fn new(m0: f32, rho_0: f32, particles: Vec<Particle>) -> Self {
    Self {
      h: 1.0,
      m0,
      rho0: rho_0,
      old_particles: particles.clone(),
      particles,
    }
  }
  pub fn particles(&self) -> &[Particle] {
    &self.particles
  }
  pub fn reset(&mut self, v: Vec<Particle>) {
    self.particles = v.clone();
    self.old_particles = v;
  }

  fn interp_at<U, Kernel, Field>(&self, kern: Kernel, location: Point3<f32>, field: Field) -> U
  where
    U: Add<U> + std::iter::Sum<U> + Mul<f32, Output = U> + num_traits::Zero,
    Kernel: Fn(f32, f32) -> f32,
    Field: Fn(&Particle) -> U,
  {
    self
      .old_particles
      .iter()
      .map(|x| {
        let r = (x.pos - location).magnitude();
        field(x) * (kern(r, self.h) / x.density)
      })
      .sum()
  }

  pub fn update(&mut self, dt: f32, k: f32, m0: f32, rho0: f32, h: f32) {
    self.m0 = m0;
    self.h = h;
    self.rho0 = rho0;

    self.old_particles = self.particles.clone();
    let mut particles = std::mem::take(&mut self.particles);

    // Solve for density and pressure at positions of particles
    let pressures: Vec<_> = particles
      .par_iter_mut()
      .map(|x| {
        // Solve
        // 1. Density
        let rho = m0
          * self
            .interp_at(kernels::poly6, x.pos, |x| x.density)
            .clamp(-10. * rho0, 10. * rho0);
        // 2. Pressure
        let mut p = k * (rho - rho0);
        if p.is_nan() {
          println!("pressure is nan!");
          0.
        } else {
          x.density = rho; //?
          p
        }
      })
      .collect();
    // Normalize pressures for symmetry
    particles.par_iter_mut().enumerate().for_each(|(i, p)| {
      p.forces = Vector3::zero();
      // p.forces.y = -0.25;
      for j in 0..pressures.len() {
        if i == j {
          continue;
        }
        p.forces -= 0.5 * (pressures[i] + pressures[j]) / p.density
          * grad_spiky(p.pos - self.old_particles[j].pos, h);
      }

      if p.forces.x.is_nan() || p.forces.y.is_nan() || p.forces.z.is_nan() {
        println!("Force is broken!");
        p.forces = Vector3::zero();
      }
      p.forces.y -= 2.0;
      // if p.forces.magnitude2() != 0. {
      //   println!("{:e}", p.forces.magnitude());
      // }
    });

    // 2. Viscosity (temporarily 0/skip)
    // 3. Surface tension
    // 4. Integrate forces
    // Move
    particles.par_iter_mut().for_each(|x| {
      let a = x.forces / x.density;
      if !x.density.is_nan() {
        x.pos += dt * x.velocity + 0.5 * a * dt * dt;
        x.velocity += dt * a;
      } else {
        println!("Density is nan");
      }
      if x.pos.to_vec().magnitude2() > 256. * 256. {
        let p = x.pos.to_vec();
        x.pos = Point3::from_vec(p.normalize() * 256.);
        x.velocity -= 1.86574 * x.velocity.project_on(p);
      }
    });
    self.particles = particles;
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
