@group(0) @binding(0)
var sphere_tex: texture_2d<f32>;
@group(0) @binding(1)
var smp: sampler;

@group(1) @binding(0)
var zbuf: texture_depth_2d;

struct VOut {
  @builtin(position) clip_pos: vec4f,
  @location(0) texcoord: vec4f
}
struct FOut {
  @builtin(frag_depth) depth: f32,
  @location(0) norm: vec4f
}

@fragment
fn fs_main(in: VOut) -> FOut {
  var o: FOut;
  o.norm = textureSample(sphere_tex, smp, in.texcoord.xy);
  o.depth = textureSample(zbuf, smp, in.texcoord.xy);
  return o;
}