struct Particle {
  pos: vec3<f32>,
  density: f32,
  velocity: vec3<f32>,
  forces: vec3<f32>,
}
struct Global {
  size: vec2<f32>,
  time: f32,
  dt: f32,
  camera: mat4x4f,
  projection: mat4x4f
}
struct SimParams {
  k: f32,
  m0: f32,
  viscosity: f32,
  h: f32,
  rho0: f32,
  e: f32,
  w: f32,
  ttr: f32,
  dtr: f32,
}

@group(0) @binding(0)
var<storage, read_write> cur_particles: array<Particle>;
@group(0) @binding(1)
var<storage, read_write> old_particles: array<Particle>;

@group(1) @binding(0)
var<storage, read_write> pressure: array<f32>;
@group(1) @binding(1)
var<storage, read> params: SimParams;

@group(2) @binding(0)
var<uniform> g: Global;


const PI: f32 = 3.14159265358979;

fn poly6(r: f32, h: f32) -> f32 {
  if 0. <= r && r <= h {
    // return 15 /PI / pow(h, 6.) * pow(h-r, 3.);
    return 315. / 64. / PI / pow(h, 9.) * pow(h * h - r * r, 3.);
  }
  return 0.;
}

fn spiky(r: f32, h: f32) -> f32 {
  if 0 <= r && r <= h {
    return 15. / (PI*h*h*h * h*h*h) * (h-r)*(h-r)*(h-r);
  }
  return 0.;
}

fn grad_spiky(r: vec3f, h: f32) -> vec3f {
  if length(r) >= h || length(r) == 0 {
    return vec3(0.);
  }
  return -45. * pow(h - length(r), 2.) / PI / pow(h, 6.) * normalize(r);
}

fn laplacian_viscosity(r: f32, h: f32) -> f32 {
  if r == 0. || r >= h {
    return 0.;
  } else {
    return (45 / PI / pow(h, 6.)) * (h - r);
  }
}

fn mpless(l: Particle, r: Particle) -> bool {
  return l.pos.x < r.pos.x;
}

fn intrp_density(at: vec3<f32>) -> f32 {
  var sum: f32 = 0.0;
  let els = arrayLength(&old_particles);
  for (var i: u32 = 0; i < els; i += u32(1)) {
    sum += poly6(distance(at, old_particles[i].pos), params.h);
  }
  sum *= params.m0;
  return sum;
}

// This constant **must** be kept the same as `solvers::sph_solver_gpu::SOLVER_WG_SIZE`
const WG_SIZE: u32 = 16;
const NA: f32 = 7;
@compute @workgroup_size(WG_SIZE)
fn density_pressure(@builtin(global_invocation_id) idx: vec3u) {
  let num = idx.x;
  // Density
  let rho = intrp_density(old_particles[num].pos);
  cur_particles[num].density = rho;
  // Pressure
  var p = 1 / (NA*params.k) * (pow((cur_particles[num].density)/params.rho0, NA) - 1);
  // var p = params.k * (rho - params.rho0);
  if p != p { // p is NaN or < 0
    p = 0.;
  }
  pressure[num] = p;
}


@compute @workgroup_size(WG_SIZE)
fn pressure_forces(@builtin(global_invocation_id) idx: vec3u) {
  let i = idx.x;
  let els = arrayLength(&pressure);
  cur_particles[i].forces = vec3f(0.);

  var probe = old_particles[i];
  probe.pos.x -= params.h;
  var l = i;
  while l > 0 && !mpless(old_particles[l], probe) {
    l -= 1u;
  }
  probe.pos.x += params.h * 2;
  var r = i;
  while r < els - 1 && !mpless(probe, old_particles[r]) {
    r += 1u;
  }
  var f_visc = vec3f(0.);
  cur_particles[i].forces = vec3(0.);
  for (var j: u32 = l; j < r; j += 1u) {
    if (i == j) {
      continue;
    }
    // pressure
    cur_particles[i].forces -= (pressure[i]/cur_particles[i].density/cur_particles[i].density
                              + pressure[j]/cur_particles[j].density/cur_particles[j].density)
                            * grad_spiky(old_particles[i].pos - old_particles[j].pos, params.h);
    // FIXME: viscosity
    f_visc += params.viscosity * (old_particles[j].velocity - old_particles[i].velocity)
     * laplacian_viscosity(distance(old_particles[i].pos, old_particles[j].pos), params.h) / cur_particles[j].density;
  }
  // NaN
  if length(cur_particles[i].forces) != length(cur_particles[i].forces) {
    cur_particles[i].forces = vec3f(0.);
  }
  cur_particles[i].forces *= cur_particles[i].density * params.m0/params.rho0;
  // External forces
  cur_particles[i].velocity += g.dt/params.m0 * (vec3f(0., -10.0, 0.) + f_visc);
  // cur_particles[i].forces += 20.0*cross(vec3(0.,1.,0.), old_particles[i].pos);
}

fn project_on(a: vec3f, direction: vec3f) -> vec3f {
  return normalize(direction) * dot(a, direction) / length(direction);
}

@compute @workgroup_size(WG_SIZE)
fn integrate_forces(@builtin(global_invocation_id) idx: vec3u) {
  let i = idx.x;
  let els = arrayLength(&pressure);
  cur_particles[i].velocity += g.dt * cur_particles[i].forces/params.m0;
  
  let a = cur_particles[i].forces / cur_particles[i].density;
  cur_particles[i].pos += g.dt * cur_particles[i].velocity + 0.5 * a * g.dt * g.dt;

  // Out of bounds check
  var p = cur_particles[i].pos;
  var v = cur_particles[i].velocity;
  let e = params.e;
  let w = params.w;
  if abs(p.z) > w {
    p.z = clamp(p.z, -w, w);
    v.z = -e * v.z;
  }
  if abs(p.x) > w {
    p.x = clamp(p.x, -w, w);
    v.x = -e * v.x;
  }
  if p.y < 0. {
    p.y = 0.;
    v.y = -e * v.y;
  }
  if p.y > 10. {
    p.y = 10.;
    v.y = -e * v.y;
  }
  cur_particles[i].pos = p;
  cur_particles[i].velocity = v;
}