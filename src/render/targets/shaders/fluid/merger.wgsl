@group(0) @binding(0)
var tex: texture_2d<f32>;
@group(0) @binding(1)
var smp: sampler;

@group(1) @binding(0)
var zbuf_smoothed: texture_depth_2d;
@group(1) @binding(1)
var normal: texture_2d<f32>;
@group(1) @binding(2)
var sphere_tex: texture_2d<f32>;
@group(1) @binding(3)
var thickness: texture_2d<f32>;

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
  @location(0) col: vec4f
}

const n1: f32 = 1.0;
const n2: f32 = 1.33;
fn specular_reflection(cos_theta: f32) -> f32 {
  var r0 = (n1-n2)/(n1+n2);
  r0 *= r0;
  return r0 + (1-r0)*pow(cos_theta, 5.);
}

const SCENE_COLOR: vec3f = vec3f(1.0, 0.0, 0.0);
const FLUID_COLOR: vec3f = vec3f(0.07, 0.075, 0.0);

@fragment
fn fs_main(in: VOut) -> FOut {
  var o: FOut;
  var diffuse = vec4f(0.);
  var specular = vec4f(0.);
  var fresnel = vec4f(0.);
  let n = normalize(textureSample(normal, smp, in.texcoord.xy));
  let v = 0.0;
  // o.col = textureSample(sphere_tex, smp, in.texcoord.xy);
  o.depth = textureSample(zbuf_smoothed, smp, in.texcoord.xy);
  // o.col = vec4(vec3(o.depth), 1.0);
  o.col = vec4(textureSample(thickness, smp, in.texcoord.xy).xxx, 1.0) * (
    1.0
  );
  return o;
}