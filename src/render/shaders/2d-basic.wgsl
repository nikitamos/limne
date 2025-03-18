struct Input {
  @builtin(vertex_index) idx: u32,
  @builtin(instance_index) iid: u32,
  @location(0) pos: vec3<f32>,
};

@group(0) @binding(0)
var <uniform> size: vec2<f32>;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
  var col: vec4<f32> = vec4(0.0, 0.0, 0.0, 1.0);
  switch (in.iid) {
    case 0u: {col.x = 1.0;}
    default: {col = vec4(0.0, 0.133, 0.5, 1.0);}
  }
  return col;
}

struct VertexOutput {
    @builtin(position) pos: vec4<f32>,
    // Instance index
    @location(0) iid: u32,
    // Vertex index
    @location(1) idx: u32
};

const delta = vec2(0.0, 20.0);
const PI = 3.1515926535898;

@vertex
fn vs_main(
  in: Input
) -> VertexOutput {
  let angle = f32(in.idx) * 2.0 * PI/3.0;
  let rot = mat2x2(
    cos(angle), sin(angle),
    -sin(angle), cos(angle)
  );
  var d = rot * delta;

  var out: VertexOutput;
  out.pos = vec4(vec2(in.pos.x, in.pos.y) + d, 0.0, 1.0);
  // if (in.idx == 0) {
    // out.pos = vec4(pos + d, 0.0, 1.0);
  // } else if (in.idx == 1) {
  //   out.pos = vec4(pos, 0.0,  1.0);
  // } else {
  //   out.pos = vec4(pos + d, 0.0, 1.0);
  // }
  out.pos.y /= size.y;
  out.pos.x /= size.x;

  out.iid = in.iid;
  out.idx = in.idx;
  return out;
}
