@group(0) @binding(0)
var normals_unsmoothed: texture_2d<f32>;
@group(0) @binding(1)
var smp: sampler;

const SIDE: i32 = 8;
const ARRAY_LENGTH: i32 = SIDE*SIDE/4;

@group(1) @binding(0)
var zbuf: texture_depth_2d;
@group(1) @binding(1)
var thickness: texture_2d<f32>;
@group(1) @binding(2)
var<uniform> kernel: array<vec4f, ARRAY_LENGTH>;

struct Global {
  size: vec2<f32>,
  time: f32,
  dt: f32,
  camera: mat4x4f,
  projection: mat4x4f
}

@group(2) @binding(0)
var<uniform> g: Global;

struct VOut {
  @builtin(position) clip_pos: vec4f,
  @location(0) texcoord: vec4f
}
struct FOut {
  @builtin(frag_depth) depth: f32,
  @location(0) norm: vec4f
}

var<private> dh: vec2f;
var<private> dx: vec2f;

const Ex: vec2f = vec2(1., 0.);
const Ey: vec2f = vec2(0., 1.);

fn sample_vec4(first: vec2f) -> vec4f {
  return vec4(
    textureSample(zbuf, smp, first),
    textureSample(zbuf, smp, first +   dx),
    textureSample(zbuf, smp, first + 2*dx),
    textureSample(zbuf, smp, first + 3*dx)
  );
}
fn get_conv(idx: vec2i) -> vec4f {
  return kernel[dot(idx, vec2(1, SIDE/4))];
}
fn convolve_quad(c: vec2f, iconv: vec2i) -> f32 {
  let pos = vec2(4*f32(iconv.x), f32(iconv.y));
  let sym_pos = -pos + Ex;
  let conv = get_conv(iconv);
  var out: f32 = 0.;

  out += dot(conv,      sample_vec4(c + pos));
  out += dot(conv,      sample_vec4(c + pos*(Ex-Ey)));
  out += dot(conv.wzyx, sample_vec4(c + sym_pos));
  out += dot(conv.wzyx, sample_vec4(c + sym_pos*(Ex-Ey)));

  return out;
}

@fragment
fn fs_main(in: VOut) -> FOut {
  var o: FOut;
  var depth = 0.;
  dx = vec2(1./g.size.x, 0.);
  let dy = vec2(0., 1./g.size.y);
  dh = dx + dy;

  for (var row: i32 = 0; row < SIDE; row += 1) {
    for (var col: i32 = 0; col < SIDE/4; col += 1) {
      depth += convolve_quad(in.texcoord.xy, vec2(col, row));
    }
  }

  o.norm = vec4f(1.0);
  o.depth = depth;
  return o;
}