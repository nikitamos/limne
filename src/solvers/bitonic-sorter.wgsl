const WG_SIZE: u32 = 1024u;
struct Particle {
  density: f32,
  pos: vec3<f32>,
  velocity: vec3<f32>,
  forces: vec3<f32>,
}
struct Params {
  offset: u32,
  h: u32,
}

var<workgroup> a: array<Particle, WG_SIZE>;
var<push_constant> p: Params;

fn do_flip(offset: u32, pair: u32) {

}

@compute @workgroup_size(WG_SIZE)
fn flip_local() {
  
}

@compute @workgroup_size(WG_SIZE)
fn disperse_local() {}

@compute @workgroup_size(WG_SIZE)
fn flip_global() {}

@compute @workgroup_size(WG_SIZE)
fn disperse_global() {}