var<workgroup> a: f32;

@compute @workgroup_size(1024)
fn flip_1024() {}

@compute @workgroup_size(1024)
fn disperse_1024() {}

@compute @workgroup_size(1)
fn global_layer() {}