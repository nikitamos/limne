@group(0) @binding(0)
var tex: texture_2d<f32>;
@group(0) @binding(1)
var smp: sampler;

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

@group(1) @binding(0)
var zbuf_smoothed: texture_depth_2d;
@group(1) @binding(1)
var normal: texture_2d<f32>;
@group(1) @binding(2)
var normals_unsmoothed: texture_2d<f32>;
@group(1) @binding(3)
var thickness: texture_2d<f32>;

@group(2) @binding(0)
var<storage,read> params: SimParams;

@group(3) @binding(0)
var<uniform> g: Global;

struct Global {
  size: vec2<f32>,
  time: f32,
  dt: f32,
  camera: mat4x4f,
  projection: mat4x4f
}

struct VOut {
  @builtin(position) clip_pos: vec4f,
  @location(0) texcoord: vec4f
}
struct FOut {
  @builtin(frag_depth) depth: f32,
  @location(0) col: vec4f
}

const n1: f32 = 1.0;
const n2: f32 = 1.33;
fn ffresnel(cos_theta: f32) -> f32 {
  var r0 = (n1-n2)/(n1+n2);
  r0 *= r0;
  return r0 + (1-r0)*pow(cos_theta, 5.);
}

const SCENE_COLOR: vec3f = vec3f(1.0, 1.0, 1.0);
const FLUID_COLOR: vec3f = vec3f(0.07, 0.075, 1.0);
const DIFFUSE_COLOR: vec3f = vec3f(1.0, 1.0, 1.0);
const SUN = vec4f(0.0, -100.0, 0.0, 1.0);

fn get_normal(pos: vec2f) -> vec3f {
  return normalize(textureSample(normal, smp, pos)).xyz*vec3(1.,1.,1.);
}

@fragment
fn fs_main(in: VOut) -> FOut {
  var o: FOut;
  o.depth = textureSample(zbuf_smoothed, smp, in.texcoord.xy);

  var n = get_normal(in.texcoord.xy);
  var v = vec3f(0.,0.,-1.);
  let t = textureSample(thickness, smp, in.texcoord.xy).x;
  let a = mix(FLUID_COLOR, SCENE_COLOR, exp(-t));
  let b = SCENE_COLOR;
  let f = ffresnel(dot(n, v));
  let sun_ = g.projection * (g.camera * SUN);
  var sun = sun_.xyz / sun_.w;

  var world = vec3(in.texcoord.xy, o.depth);
  world.y = 1. - world.y;
  world = vec3(2.,2.,1.)*world - vec3(1.,1.,0.);
  var ray = normalize(world - sun);

  let specular = pow(dot(n, ray), 5.0);
  if (t < params.ttr) {
    discard;
  }
  let diffuse = dot(n, ray);

  let phong =
    a * (1 - f)
  // + b * f
  + DIFFUSE_COLOR*diffuse;
  o.col = vec4(phong, 1.0);
  return o;
}