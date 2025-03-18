struct Input {
  @builtin(vertex_index) idx: u32,
  @builtin(instance_index) iid: u32,
  @location(0) pos: vec3<f32>,
};
struct VertexOutput {
    @builtin(position) pos: vec4<f32>,
    // Vertex index
    @location(0) idx: u32,
    @location(1) iid: u32,
};

@group(0) @binding(0)
var <uniform> size: vec2<f32>;

struct Cell {
  vx: f32,
  vy: f32,
  vz: f32,
  pressure: f32,
  density: f32
};

struct Grid {
  // grid: vec2<u32>,
  w: u32,
  h: u32,
  cell_side: f32
};

@group(1) @binding(0)
var<storage> cells: array<Cell>;
@group(1) @binding(1)
var<storage> grid: Grid;

fn get_cell(world_pos: vec2<f32>) -> Cell {
  var c: Cell;
  var ind = clamp(vec2<u32>((world_pos + size) / grid.cell_side), vec2(0u, 0u), vec2(grid.w, grid.h) - vec2(1u, 1u));

  return cells[ind.x + ind.y * grid.w];
}

const COUNT = 500.0;
@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
  var col: vec4<f32> = vec4(0.0, 0.133, 0.4, 1.0);
  let inst = f32(in.iid);
  // var safd: vec4<f32> = clamp((inst / COUNT) * col, vec4(0.0,0.0,0.0,1.0), vec4(1.0,1.0,1.0,1.0));
  // if (in.idx != 0) {
  //   safd = vec4<f32>(0.0,1.0,0.0,1.0);
  // }
  
  return col;
}


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

  let cell = get_cell(vec2(in.pos.x, in.pos.y));
  
  var speed_angle: f32 = 0;
  if (cell.vy == 0 && cell.vx <= 0.0) {
    speed_angle = PI;
  } else if (cell.vx < 0.0) {
    speed_angle = PI + atan(cell.vx / cell.vy);
  } else {
    speed_angle = atan(cell.vx / cell.vy);
  }

  let rot2 = mat2x2(
    cos(speed_angle), sin(speed_angle),
    -sin(speed_angle), cos(speed_angle)
  );
  var d = (rot * rot2) * delta;
  if (in.idx == 0) {
    d *= 2.0;
  }

  var out: VertexOutput;
  out.pos = vec4(vec2(in.pos.x, in.pos.y) + d, 0.0, 1.0);
  out.pos.y /= size.y;
  out.pos.x /= size.x;

  out.idx = in.idx;
  out.iid = in.iid;
  return out;
}
