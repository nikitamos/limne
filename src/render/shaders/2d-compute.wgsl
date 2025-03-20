struct Global {
  size: vec2<f32>,
  time: f32,
  dt: f32
};

struct Cell {
  vx: f32,
  vy: f32,
  vz: f32,
  pressure: f32,
  density: f32
};

@group(0) @binding(0)
var<uniform> g: Global;

struct Grid {
  // grid: vec2<u32>,
  w: u32,
  h: u32,
  cell_side: f32
};

@group(1) @binding(0)
var<storage> cur_cells: array<Cell>;
@group(1) @binding(1)
var<storage> grid: Grid;

fn get_cell(world_pos: vec2<f32>) -> Cell {
  var c: Cell;
  var ind = clamp(vec2<u32>(world_pos / grid.cell_side), vec2(0u, 0u), vec2(grid.w, grid.h) - vec2(1u, 1u));

  return cur_cells[ind.x + ind.y * grid.w];
}

fn cell_idx(world_pos: vec2<f32>) -> u32 {
  let ind =  clamp(vec2<u32>(world_pos / grid.cell_side), vec2(0u, 0u), vec2(grid.w, grid.h) - vec2(1u, 1u));
  return ind.x + ind.y * grid.w;
}

// fn old_cell(world_pos: vec2<f32>) -> Cell {
//   var c: Cell;
//   var ind = clamp(vec2<u32>(world_pos / grid.cell_side), vec2(0u, 0u), vec2(grid.w, grid.h) - vec2(1u, 1u));

//   return old_cells[ind.x + ind.y * grid.w];
// }

struct Vec3 {
  x: f32,
  y: f32,
  z: f32
}

@group(2) @binding(0)
var<storage, read_write> positions: array<Vec3>;
@group(2) @binding(1)
var<storage, read_write> old_positions: array<Vec3>;


@compute @workgroup_size(1)
fn apply_velocities(@builtin(global_invocation_id) inv_id: vec3<u32>) {
  let i = inv_id.x;
  let c = cur_cells[cell_idx(vec2(positions[i].x, positions[i].y))];
  positions[i].x += g.dt * c.vx;
  positions[i].y += g.dt * c.vy;
}
@compute @workgroup_size(1)
fn step() {

}