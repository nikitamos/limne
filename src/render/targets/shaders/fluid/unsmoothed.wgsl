struct Input {
  @builtin(vertex_index) idx: u32,
  @builtin(instance_index) iid: u32,
  @location(0) pos: vec3<f32>,
  @location(1) rho: f32,
};
struct VertexOutput {
  @builtin(position) out_clip_pos: vec4<f32>,
  @location(0) center_pos: vec3<f32>,
  // Vertex index
  @location(1) eye_pos: vec4<f32>,
  @location(2) clip_pos : vec4f,
  @location(3) rho: f32
}
struct FragmentOutput {
  @location(0) col: vec4f,
  @location(1) view_pos: vec4f,
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
  e: f32,
  w: f32,
  ttr: f32,
  dtr: f32
}

// BINDING BEGIN

@group(0) @binding(0)
var<uniform> g: Global;

@group(1) @binding(0)
var<storage,read> params: SimParams;
// BINDINGS END

@fragment
fn depth_normals(in: VertexOutput) -> FragmentOutput {
  var out: FragmentOutput;
  if in.rho < params.rho0*params.dtr {
    discard;
  }
  
  let center_eye = g.camera * vec4(in.center_pos, 1.0);
  let radius = params.h;
  let r = (in.eye_pos - center_eye).xy;
  let r2 = dot(r, r);
  if (r2 >= radius * radius) {
    discard;
  }
  var n = vec3(r, sqrt(radius*radius - r2));
  let pixel_pos = (center_eye + vec4f(n, 0.0));
  let clip_pos = g.projection * pixel_pos;

  out.depth = clip_pos.z / clip_pos.w;
  n.z = -n.z;
  out.col = vec4f(normalize(n), 1.0);
  out.view_pos = vec4f(0.0, pixel_pos.xyz);
  
  return out;
}

@fragment
fn thickness(in: VertexOutput) -> @location(0) vec4f {
  var out: f32;
  if in.rho < params.rho0*params.dtr {
    discard;
  }
  
  let center_eye = g.camera * vec4(in.center_pos, 1.0);
  let radius = params.h;
  let r = (in.eye_pos - center_eye).xy;
  let r2 = dot(r, r);
  if (r2 >= radius * radius) {
    discard;
  }
  let n = vec3(r, sqrt(radius*radius - r2));
  let pixel_pos = (center_eye + vec4f(n, 0.0));
  let clip_pos = g.projection * pixel_pos;
  let back_eye = pixel_pos - 2*vec4(0., 0., pixel_pos.z, 0.0);
  let back_clip = g.projection * back_eye;
  out = abs(clip_pos.z / clip_pos.w - back_clip.z / back_clip.w);
  return vec4(out, 0.0, 0.0, 0.0);
}


const delta = vec2(5.0, 0.0);
const PI = 3.1515926535898;
const SQRT_3 = 1.7320508076;

@vertex
fn vs_main(
  in: Input
) -> VertexOutput {
  let angle = f32(in.idx) * 2.0 * PI/3.0;

  let rot = mat2x2(
    cos(angle), sin(angle),
    -sin(angle), cos(angle)
  );
  var d = rot * vec2(params.h * SQRT_3*2., 0.0);

  var out: VertexOutput;
  out.eye_pos = g.camera * vec4(in.pos, 1.0) + vec4(d.x, d.y, 0.0, 0.0);
  out.clip_pos = g.projection * out.eye_pos;
  out.center_pos = in.pos;
  out.out_clip_pos = out.clip_pos;
  out.rho = in.rho;
  return out;
}