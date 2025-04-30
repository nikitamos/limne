@group(0) @binding(0)
var tex: texture_2d<f32>;
@group(0) @binding(1)
var smp: sampler;

struct VOut {
  @builtin(position) clip_pos: vec4f,
  @location(0) texcoord: vec4f
}

@fragment
fn fs_main(in: VOut) -> @location(0) vec4f {
    // discard;
  return textureSample(tex, smp, in.texcoord.xy); //vec4(1., 0., 0., 1.);
}