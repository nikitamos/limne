struct Input {
  @builtin(vertex_index) idx: u32,
  @builtin(instance_index) iid: u32,
  @location(0) pos: vec3<f32>,
};
struct VertexOutput {
    @builtin(position) pos: vec4<f32>,
    // Vertex index
    @location(0) particle_pos: vec3<f32>,
    @location(1) idx: u32,
    @location(2) iid: u32,
};

struct Global {
  size: vec2<f32>,
  time: f32,
  dt: f32,
  camera: mat4x4f
};

// BINDING BEGIN

@group(0) @binding(0)
var<uniform> g: Global;

// BINDINGS END

fn lerp(a1: f32, a2: f32,
        b1: f32, b2: f32,
        a: f32)-> f32 {
  return b1 + (b2 - b1) * saturate((a - a1) / (a2 - a1));
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
  var col: vec4<f32> = mix(vec4(1.0, 0.0, 0.0, 1.0), vec4(0., 0., 1., 1.), length(in.particle_pos)/400.);
  // let inst = f32(in.iid);
  
  return col;
}


const delta = vec2(5.0, 0.0);
const PI = 3.1515926535898;

@vertex
fn vs_main(
  in: Input
) -> VertexOutput {
  let angle = f32(in.idx) * 2.0 * PI/3.0;
  
  var speed_angle: f32 = 0;
  // if (cell.vy == 0) {
  //   speed_angle = PI/2 * (1 - sign(cell.vx));
  // } else if (cell.vx >= 0.0) {
  //   if (cell.vy >= 0) { // I
  //     speed_angle = PI/2. -  atan(cell.vx / cell.vy);
  //   } else { // IV
  //     speed_angle = -PI/2 - atan(cell.vx / cell.vy);
  //   }
  // } else {
  //   if (cell.vy >= 0.){ // II
  //     speed_angle = PI/2. - atan(cell.vx / cell.vy);
  //   } else { // III
  //     speed_angle = -PI/2 - atan(cell.vx / cell.vy);
  //   }
  // }

  let rot = mat2x2(
    cos(speed_angle+angle), sin(speed_angle+angle),
    -sin(speed_angle+angle), cos(speed_angle+angle)
  );
  var d = rot * delta;
  if (in.idx == 0) {
    d *= 2.0;
  }

  var out: VertexOutput;
  out.pos = g.camera * vec4(in.pos.x + d.x, in.pos.y + d.y, in.pos.z, 1.0);

  out.particle_pos = in.pos;
  out.idx = in.idx;
  out.iid = in.iid;
  return out;
}