struct Input {
  @builtin(vertex_index) idx: u32,
  @builtin(instance_index) iid: u32,
  @location(0) pos: vec3<f32>
};

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
  return vec4<f32>(0.0, 0.133, 0.4, 0.5);
}

struct VertexOutput {
    @builtin(position) pos: vec4<f32>,
    @location(0) id: u32
    // @location(0) position: vec2<f32>
};

const delta = vec2(0.0, 0.05);
const PI = 3.1515926535898;

@vertex
fn vs_main(
  in: Input
) -> VertexOutput {
  let a = PI/3.0;
  let rot = mat2x2(
    sin(a), -cos(a),
    cos(a), sin(a)
  );
  var d = delta;

  for (var i = 1u; i < in.idx; i += 1u) {
    d = rot * delta;
  }

  var out: VertexOutput;
  let pos = vec2(in.pos.x, in.pos.y);
  if (in.idx == 0) {
    out.pos = vec4(pos + d, 0.0, 1.0);
  } else if (in.idx == 1) {
    out.pos = vec4(pos, 0.0,  1.0);
  } else {
    out.pos = vec4(pos + d, 0.0, 1.0);
  }
  out.id = in.iid;
  return out;
}
