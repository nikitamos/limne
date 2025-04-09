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

struct Grid {
  w: u32,
  h: u32,
  cell_side: f32,
  vmin: f32,
  vmax: f32
};

struct OtherParams {
  K: f32,
  m0: f32,
};

// Grid (dimensions, cell sides)
@group(1) @binding(0)
var<storage, read_write> grid: Grid;

// Global, obviously
@group(0) @binding(0)
var<uniform> g: Global;

// Positions
@group(2) @binding(0)
var<storage, read_write> positions: array<Vec3>;
@group(2) @binding(1)
var<storage, read_write> old_positions: array<Vec3>;

// New and all cells themselves (speed, density, pressure)
@group(3) @binding(0)
var<storage, read_write> cur_cells: array<Cell>;
@group(3) @binding(1)
var<storage, read_write> old_cells: array<Cell>;

@group(4) @binding(0)
var<storage, read> params: OtherParams;

fn get_cell(world_pos: vec2<f32>) -> Cell {
  var c: Cell;
  var ind = clamp(vec2<u32>(world_pos / grid.cell_side), vec2(0u, 0u), vec2(grid.w, grid.h) - vec2(1u, 1u));

  return cur_cells[ind.x + ind.y * grid.w];
}

fn cell_idx(world_pos: vec2<f32>) -> u32 {
  let ind =  clamp(vec2<u32>(world_pos / grid.cell_side), vec2(0u, 0u), vec2(grid.w, grid.h) - vec2(1u, 1u));
  return ind.x + ind.y * grid.w;
}

fn old_cell(world_pos: vec2<f32>) -> Cell {
  var c: Cell;
  var ind = clamp(vec2<u32>(world_pos / grid.cell_side), vec2(0u, 0u), vec2(grid.w, grid.h) - vec2(1u, 1u));

  return old_cells[ind.x + ind.y * grid.w];
}

fn get_idx(x: u32, y: u32) -> u32 {
  return x + y * grid.w;
}

fn old_cell_at(x: u32, y: u32) -> Cell {
  return old_cells[get_idx(x, y)];
}

struct Vec3 {
  x: f32,
  y: f32,
  z: f32
}

@compute @workgroup_size(1)
fn apply_velocities(@builtin(global_invocation_id) inv_id: vec3<u32>) {
  let i = inv_id.x;
  let old_idx = cell_idx(vec2(positions[i].x, positions[i].y));
  let c = cur_cells[old_idx];

  positions[i].x += g.dt * c.vx;
  positions[i].y += g.dt * c.vy;

  // **Do not update densities to solve Navier-Stokes as specified in an article**
  // !!! THREAD-UNSAFE!!!
  let new_idx = cell_idx(vec2(positions[i].x, positions[i].y));
  // if (new_idx != old_idx) {
  //   cur_cells[new_idx].density += params.m0 / grid.cell_side / grid.cell_side;
  //   cur_cells[old_idx].density -= params.m0 / grid.cell_side / grid.cell_side;
  // }
}

@compute @workgroup_size(1)
fn mass_conservation(@builtin(global_invocation_id) inv_id: vec3<u32>) {
  let index = get_idx(inv_id.x, inv_id.y);

  {
    let x_border = inv_id.x == 0 || inv_id.x == grid.w - 1;
    let y_border = inv_id.y == 0;
    if (x_border || y_border) {   
      cur_cells[index].vx = 0.0;
      cur_cells[index].vy = 0.0;
      if (inv_id.x == 0)
      {cur_cells[index].density = 5.0;}
      return;
    }
  }

  let h = grid.cell_side;
  let cur_cell = cur_cells[index];
  let old_cell = old_cells[index];
  
  let top = old_cell_at(inv_id.x, inv_id.y+1);
  let bottom = old_cell_at(inv_id.x, inv_id.y-1);
  let left = old_cell_at(inv_id.x-1, inv_id.y);
  let right = old_cell_at(inv_id.x+1, inv_id.y);

  let u = vec2(cur_cell.vx, cur_cell.vy);
  let u_top = vec2(top.vx, top.vy);
  let u_left = vec2(left.vx, left.vy);
  let u_right = vec2(right.vx, right.vy);
  let u_bottom = vec2(bottom.vx, bottom.vy);

  let du_dx = (u_right - u_left) / h;
  let du_dy = (u_top - u_bottom) / h;
  let div_u = du_dx.x + du_dy.y;

  let grad_rho =
       vec2(right.density - left.density,
            top.density - bottom.density) / 2. / h;
  var rho = old_cell.density - g.dt * (dot(grad_rho, u) + old_cell.density*div_u);
  // CLAMP DENSITY
  rho = clamp(rho, 0.5, 3.0);

  let S = params.K / g.dt;
  let grad_p = grad_rho * S;
  let laplacian = mat4x2f(u_right, u_left, u_top, u_bottom) * vec4f(1.0);

  let pos = vec2(f32(inv_id.x) * h, f32(inv_id.y) *h) + 0.5*h;
  let external_force = vec2(pos.y, -pos.x);
  
  let du = g.dt * (0.0*laplacian - grad_p + 2.0*h*normalize(external_force));

  cur_cells[index].density = rho;
  cur_cells[index].vx += du.x;
  cur_cells[index].vy += du.y;
}