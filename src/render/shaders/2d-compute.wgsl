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
var<storage> old_cells: array<Cell>;
@group(1) @binding(2)
var<storage> grid: Grid;

fn get_cell(world_pos: vec2<f32>) -> Cell {
  var c: Cell;
  var ind = clamp(vec2<u32>(world_pos / grid.cell_side), vec2(0u, 0u), vec2(grid.w, grid.h) - vec2(1u, 1u));

  return cur_cells[ind.x + ind.y * grid.w];
}

fn old_cell(world_pos: vec2<f32>) -> Cell {
  var c: Cell;
  var ind = clamp(vec2<u32>(world_pos / grid.cell_side), vec2(0u, 0u), vec2(grid.w, grid.h) - vec2(1u, 1u));

  return old_cells[ind.x + ind.y * grid.w];
}


@compute @workgroup_size(1)
fn apply_velocities() {

}
@compute @workgroup_size(1)
fn step() {

}