@group(0) @binding(0)
var normals_unsmoothed: texture_2d<f32>;
@group(0) @binding(1)
var smp: sampler;

const SIDE: i32 = 24;
const CENTER: vec2i = vec2(SIDE, SIDE);
const DIM_LEN: i32 = 2*SIDE+1;
const ARRAY_LEN: i32 = DIM_LEN*DIM_LEN;

@group(1) @binding(0)
var zbuf: texture_depth_2d;
@group(1) @binding(1)
var thickness: texture_2d<f32>;
@group(1) @binding(2)
var<storage> kernel: array<f32, ARRAY_LEN>;

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

fn at(i: vec2i) -> f32 {
  return kernel[i.x + i.y*DIM_LEN];
}

@fragment
fn fs_main(in: VOut) -> FOut {
  var o: FOut;
  o.depth = 0.;
  dx = vec2(1./g.size.x, 0.);
  let dy = vec2(0., 1./g.size.y);
  dh = dx + dy;

  var px = vec2(-SIDE, -SIDE);
  for (; px.x < SIDE; px.x += 1) {
    for (px.y = -SIDE; px.y < SIDE; px.y += 1) {
      let pos = vec2f(px);
      o.depth += textureSample(zbuf, smp, in.texcoord.xy + dh*pos) * at(px+CENTER);
      o.norm += textureSample(normals_unsmoothed, smp, in.texcoord.xy + dh*pos) * at(px+CENTER);
    };
  }
  o.norm.w = textureSample(normals_unsmoothed, smp, in.texcoord.xy).w;
  return o;
}