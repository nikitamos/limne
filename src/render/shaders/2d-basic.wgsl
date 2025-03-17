struct Input {
  @location(0) positions: vec3<f32>
};

struct VertexOutput {
  @builtin(position) pos: vec4<f32>
}

@vertex
fn vs_main(@builtin(vertex_index) in_v_index: u32) -> VertexOutput {
  let pos = vec3(0.0, 0.0, 0.0) /* retrieve from the buffer */;
  const delta = 0.002;
  var out: VertexOutput;
  if (in_v_index % 3 == 0) {
    out.pos = vec4(pos + vec3(0.0,delta,0.0), 1.0);
  } else if (in_v_index % 3 == 1) {
    out.pos = vec4(pos+vec3(-delta, -delta, 0.0), 1.0);
  } else {
    out.pos = vec4(pos+vec3(0.0, -delta, -delta), 1.0);
  }
  return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(1) vec4<f32> {
  return vec4<f32>(0.0, 0.133, 0.4, 1.0);
}