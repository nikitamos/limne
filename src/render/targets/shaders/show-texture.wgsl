@group(0) @binding(0)
var tex: texture_2d<f32>;
@group(0) @binding(1)
var smp: sampler;

struct VOut {
  @builtin(position) clip_pos: vec4f,
  @location(0) true_pos: vec4f
}

@vertex
fn vs(@builtin(vertex_index) idx: u32) -> VOut {
  var out: VOut;
  if (idx == 0) {
    out.clip_pos = vec4(-1., -1., 0., 1.);
  } else if (idx == 1) {
    out.clip_pos = vec4(-1., 1., 0., 1.);
  } else if (idx == 2) {
    out.clip_pos = vec4(1., -1., 0., 1.);
  } else {
    out.clip_pos = vec4(1., 1., 0., 1.);
  }
  out.true_pos = out.clip_pos;
  out.true_pos *= .5;
  out.true_pos += .5;
  out.true_pos.y = 1. - out.true_pos.y;
  return out;
}

@fragment
fn fs(in: VOut) -> @location(0) vec4f {
  return textureSample(tex, smp, in.true_pos.xy);
}