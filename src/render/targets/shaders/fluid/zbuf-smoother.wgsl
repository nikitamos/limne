@group(0) @binding(0)
var normals_unsmoothed: texture_2d<f32>;
@group(0) @binding(1)
var smp: sampler;

@group(1) @binding(0)
var zbuf: texture_depth_2d;
@group(1) @binding(1)
var thickness: texture_2d<f32>;

struct Global {
  size: vec2<f32>,
  time: f32,
  dt: f32,
  camera: mat4x4f,
  projection: mat4x4f
}

@group(2) @binding(0)
var<uniform> g: Global;

struct VOut {
  @builtin(position) clip_pos: vec4f,
  @location(0) texcoord: vec4f
}
struct FOut {
  @builtin(frag_depth) depth: f32,
  @location(0) norm: vec4f
}

const STEPS: i32 = 20;
const dt: f32 = 0.1/2.;

fn dTex_dx(pos: vec2f, tex: texture_2d<f32>) -> vec4f {
  let h = vec2(1./g.size.x, 0.);
  let r = textureSample(tex, smp, pos + h);
  let l = textureSample(tex, smp, pos - h);
  return (r - l) / 2. / h.x;
}
fn dTex_dy(pos: vec2f, tex: texture_2d<f32>) -> vec4f {
  let h = vec2(0., 1./g.size.y);
  let r = textureSample(tex, smp, pos + h);
  let l = textureSample(tex, smp, pos - h);
  return (r - l) / 2. / h.y;
}

@fragment
fn fs_main(in: VOut) -> FOut {
  let fx = g.projection[0][0];
  let fy = g.projection[1][1];
  let vx = g.size.x;
  let vy = g.size.y;
  let cx = 2. / vx / fx;
  let cy = 2. / vy / fy;

  var o: FOut;
  var depth = -textureSample(thickness, smp, in.texcoord.xy).w;
  let dh = vec2(1.) / g.size;

  var dzdx: f32;
  var dzdy: f32;
  var ex: f32;
  var ey: f32;
  var d: f32;
  var H: f32;
  // TODO: use vectorization (dot product) where possible
  for (var i = 0; i<STEPS; i += 1) {
    dzdx = dpdxFine(depth) / dh.x;
    dzdy = dpdyFine(depth) / dh.y;
    let d2zdx2 = dpdxFine(dzdx);
    let d2zdy2 = dpdyFine(dzdy);
    let d2z = .5* (dpdxFine(dzdy)/dh.y + dpdyFine(dzdx)/dh.x);

    d = cy*cy*dzdx*dzdx + cx*cx*dzdy*dzdy + cx*cx*cy*cy*depth*depth;
    ex = 0.5 * dzdx * (2*cy*cy*dzdx*d2zdx2 + 2*cx*cx*dzdy*d2z + 2*cx*cx*cy*cy*depth*dzdx)
         - d2zdx2 * d;
    ey = 0.5 * dzdy * (2*cy*cy*dzdx*d2z + 2*cx*cx*dzdy*d2zdy2 + 2*cx*cx*cy*cy*depth*dzdy)
         - d2zdy2 * d;
    H = (cy*ex + cx*ey) / pow(d, 1.5) / 2.;
    depth += H*dt;
  }
  
  // let dx = dpdxFine(depth);
  // let dy = dpdyFine(depth);
  // let H = 0.0;//w dx + dy;

  // let normal = textureSample(normals_unsmoothed, smp, in.texcoord.xy).xyz;
  let normal = -normalize(vec3f(-cy*ex, -cx*ey, cx*cy*depth));
  o.norm = vec4f(normal, H);
  depth = textureSample(zbuf, smp, in.texcoord.xy);
  o.depth = depth;
  return o;
}