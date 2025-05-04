const WG_SIZE: u32 = 512;
const LOCAL_ARRAY_LEN: u32 = WG_SIZE * 2;

struct Particle {
  pos: vec3<f32>,
  density: f32,
  velocity: vec3<f32>,
  forces: vec3<f32>,
}

var<workgroup> local: array<Particle, LOCAL_ARRAY_LEN>;

@group(0) @binding(0)
var<storage, read_write> cur_particles: array<Particle>;
@group(0) @binding(1)
var<storage, read_write> old_particles: array<Particle>;

fn local_cas(l: u32, r: u32) {
  if local[l].density > local[r].density {
    let buf = local[l];
    local[l] = local[r];
    local[r] = buf;
  }
}

/// q = log(height)
/// i = the number of operator
fn do_disperse_local_stage(q: u32, i: u32) {
  // 2^q is the height of disperse block in the stage
  let dbh = 1u << q;
  let dib = 2u * i / dbh;
  let go = dib * dbh;
  let jj = i % (dbh/2);
  local_cas(go + jj, go + jj + dbh / 2);
}

@compute @workgroup_size(WG_SIZE)
fn flip_local(
  @builtin(global_invocation_id) gii : vec3<u32>,
  @builtin(local_invocation_id) lii: vec3<u32>,
  @builtin(workgroup_id) wid: vec3<u32>) {
  let i = gii.x;
  let j = lii.x;
  local[2*j] = cur_particles[LOCAL_ARRAY_LEN*wid.x + 2*j];
  local[2*j+1] = cur_particles[LOCAL_ARRAY_LEN*wid.x + 2*j + 1];
  workgroupBarrier();
  
  let n = LOCAL_ARRAY_LEN;
  let k = countTrailingZeros(n);
  for (var _t: u32 = 0; _t <= k-1; _t += 1u) {
    let t = k-1 - _t;
    // 2^t is count of flip blocks
    // height of a flip block?
    let flh = 1u << (k - t);
    // block num = j / (ops per block) = j / (height/2) = 2j / height
    let flb = 2u * j / flh;
    // operation number
    let lo = j % (flh/2);
    // offset of block
    let go = flh * flb;
    local_cas(go + lo, go + flh - lo - 1);
    workgroupBarrier();

    for (var _q: u32 = 1; _q <= k - t; _q += 1u) {
      let q = k - t - _q;
      do_disperse_local_stage(q, j);
      workgroupBarrier();
    }
  }
  cur_particles[LOCAL_ARRAY_LEN*wid.x + 2*j] = local[2*j];
  cur_particles[LOCAL_ARRAY_LEN*wid.x + 2*j + 1] = local[2*j+1];
}

@compute @workgroup_size(WG_SIZE)
fn disperse_local(
  @builtin(local_invocation_id) lii: vec3<u32>,
  @builtin(workgroup_id) wid: vec3<u32>
) {
  let j = lii.x;
  local[2*j] = cur_particles[LOCAL_ARRAY_LEN*wid.x + 2*j];
  local[2*j+1] = cur_particles[LOCAL_ARRAY_LEN*wid.x + 2*j + 1];
  workgroupBarrier();

  let k = countTrailingZeros(LOCAL_ARRAY_LEN);
  for (var _q: u32 = 0; _q <= k; _q += 1u) {
    let q = k - _q;
    do_disperse_local_stage(q, j);
    workgroupBarrier();
  }

  cur_particles[LOCAL_ARRAY_LEN*wid.x + 2*j] = local[2*j];
  cur_particles[LOCAL_ARRAY_LEN*wid.x + 2*j + 1] = local[2*j+1];
}