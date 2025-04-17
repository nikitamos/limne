struct Input {
  @builtin(vertex_index) idx: u32,
  @builtin(instance_index) iid: u32,
  @location(0) pos: vec3<f32>,
};

struct VertexOutput {
    @builtin(position) pos: vec4<f32>,
    @location(0) iid: u32
};

struct Global {
  size: vec2<f32>,
  time: f32,
  dt: f32,
  camera: mat4x4f
};

@group(0) @binding(0)
var<uniform> g: Global;

@vertex
fn vs_main(in: Input) -> VertexOutput {
  var out: VertexOutput;
  out.iid = in.iid;
  var rotation = mat3x3f(
    1., 0., 0.,
    0., 1., 0.,
    0., 0., 1,
  );
  if (in.iid == 0) { // X axis
  } else if (in.iid == 1) { // Y axis
    rotation = mat3x3f(
      0., 1., 0.,
      -1., 0., 0.,
      0., 0., 1.
    );
  } else if (in.iid == 2) { // Z axis
    rotation = mat3x3f(
      0., 0., -1.,
      0., 1., 0.,
      1, 0., 0.
    );
  }
  var p = in.pos;
  p.z = -in.pos.z;
  out.pos = g.camera * vec4(rotation * p, 1.0);
  return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4f {
  if (in.iid == 0) { // Red for X
    return vec4(1.0, 0.0, 0.0, 1.0);
  } else if (in.iid == 1) { // Green for Y
    return vec4(0.0, 1.0, 0.0, 1.0);
  } else { // Blue for Z
    return vec4(0.0, 0.0, 1.0, 1.0);
  }
}