const WG_SIZE: u32 = 64u;

struct Params {
  k: u32,
  tq: u32
}
struct Particle {
  pos: vec3<f32>,
  density: f32,
  velocity: vec3<f32>,
  forces: vec3<f32>,
}

@group(0) @binding(0)
var<storage, read_write> cur_particles: array<Particle>;
@group(0) @binding(1)
var<storage, read_write> old_particles: array<Particle>;

var<push_constant> p: Params;

fn global_cas(l: u32, r: u32) {
  if cur_particles[l].pos.x > cur_particles[r].pos.x {
    let buf = cur_particles[l];
    cur_particles[l] = cur_particles[r];
    cur_particles[r] = buf;
  }
}

@compute @workgroup_size(WG_SIZE)
fn flip_global(@builtin(global_invocation_id) gii: vec3<u32>) {
  let j = gii.x;
  let flh = 1u << (p.k - p.tq);
  // block num = j / (ops per block) = j / (height/2) = 2j / height
  let flb = 2u * j / flh;
  // operation number
  let lo = j % (flh/2);
  // offset of block
  let go = flh * flb;
  global_cas(go + lo, go + flh - lo - 1);
}

// Performs one 'stage' of disperse in global memory
@compute @workgroup_size(WG_SIZE)
fn disperse_global(@builtin(global_invocation_id) gii: vec3<u32>) {
  let i = gii.x;
  let dbh = 1u << p.tq;
  let dib = 2u * i / dbh;
  let go = dib * dbh;
  let jj = i % (dbh/2);
  global_cas(go + jj, go + jj + dbh / 2);
}