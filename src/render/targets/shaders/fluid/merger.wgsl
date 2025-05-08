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

@fragment
fn fs_main(in: VOut) -> FOut {
  var o: FOut;
  // o.col = textureSample(sphere_tex, smp, in.texcoord.xy);
  o.depth = textureSample(zbuf_smoothed, smp, in.texcoord.xy);
  // o.col = vec4(vec3(o.depth), 1.0);
  o.col = vec4(textureSample(thickness, smp, in.texcoord.xy).xxx, 1.0);
  return o;
}