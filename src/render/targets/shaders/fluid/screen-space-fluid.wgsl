struct Input {
  @builtin(vertex_index) idx: u32,
  @builtin(instance_index) iid: u32,
  @location(0) pos: vec3<f32>,
};
struct VertexOutput {
    @builtin(position) out_clip_pos: vec4<f32>,
    @location(0) center_pos: vec3<f32>,
    // Vertex index
    @location(1) eye_pos: vec4<f32>,
    @location(2) clip_pos : vec4f
}
struct FragmentOutput {
  @location(0) col: vec4f,
  @builtin(frag_depth) depth: f32,
}

struct Global {
  size: vec2<f32>,
  time: f32,
  dt: f32,
  camera: mat4x4f,
  projection: mat4x4f
};
struct SimParams {
  k: f32,
  m0: f32,
  viscosity: f32,
  h: f32,
  rho0: f32,
}

struct FIn {
  @builtin(position) clip_pos: vec4f,
  @location(0) true_pos: vec4f
}

// BINDING BEGIN

@group(0) @binding(0)
var tex: texture_depth_2d;
@group(0) @binding(1)
var smp: sampler;

@group(1) @binding(0)
var<uniform> g: Global;

@group(2) @binding(0)
var<storage,read> params: SimParams;
// BINDINGS END

@fragment
fn fs(in: FIn) -> @location(0) vec4f {
  return vec4(0.);
}
