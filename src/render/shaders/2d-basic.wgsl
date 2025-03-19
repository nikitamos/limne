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
  dt: f32
};

@group(0) @binding(0)
var<uniform> g: Global;

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
  var ind = clamp(vec2<u32>(world_pos / grid.cell_side), vec2(0u, 0u), vec2(grid.w, grid.h) - vec2(1u, 1u));

  return cells[ind.x + ind.y * grid.w];
}

fn lerp(a1: f32, a2: f32,
        b1: f32, b2: f32,
        a: f32)-> f32 {
  return b1 + (b2 - b1) * saturate(a / (a2 - a1));
}

const MIN_VELOCITY = 200.0;
const MAX_VELOCITY = 1500.0;
@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
  // let cell = get_cell(vec2(in.pos.x, in.pos.y));
  let cell = get_cell(vec2(in.particle_pos.x, in.particle_pos.y));
  let len = length(vec2(cell.vx, cell.vy));
  let r = lerp(MIN_VELOCITY, MAX_VELOCITY, 0.0, 1.0, len);

  var col: vec4<f32> = vec4(r, 0.0, 1.0 - r, 1.0);
  let inst = f32(in.iid);
  
  return col;
}


const delta = vec2(5.0, 0.0);
const PI = 3.1515926535898;

@vertex
fn vs_main(
  in: Input
) -> VertexOutput {
  let angle = f32(in.idx) * 2.0 * PI/3.0;

  let cell = get_cell(vec2(in.pos.x, in.pos.y));
  
  var speed_angle: f32 = 0;
  let v_len = length(vec2(cell.vx, cell.vy));
  if (cell.vy == 0) {
    speed_angle = PI/2 * (1 - sign(cell.vx));
  } else if (cell.vx >= 0.0) {
    if (cell.vy >= 0) { // I
      speed_angle = PI/2. -  atan(cell.vx / cell.vy);
    } else { // IV
      speed_angle = -PI/2 - atan(cell.vx / cell.vy);
    }
  } else {
    if (cell.vy >= 0.){ // II
      speed_angle = PI/2. - atan(cell.vx / cell.vy);
    } else { // III
      speed_angle = -PI/2 - atan(cell.vx / cell.vy);
    }
  }

  let rot = mat2x2(
    cos(speed_angle+angle), sin(speed_angle+angle),
    -sin(speed_angle+angle), cos(speed_angle+angle)
  );
  var d = rot * delta;
  if (in.idx == 0) {
    d *= 2.0;
  }

  var out: VertexOutput;

  // Да будут вовек мучены в преисподней грешныя
  // души еретиков, воеже безцельнаго надругания
  // над математикой строки co столбцы матриц преместиша.

  let world_to_clip = transpose(mat3x3(
    2.0 / g.size.x,       0.0,      -1.0,
        0.0,        2.0 / g.size.y, -1.0,
        0.0,            0.0,      0.0
  ));
  
  out.pos = vec4(world_to_clip * vec3(in.pos.x+d.x, in.pos.y+d.y, 1.0), 1.0);

  out.particle_pos = in.pos;
  out.idx = in.idx;
  out.iid = in.iid;
  return out;
}
