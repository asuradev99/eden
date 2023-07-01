struct Particle {
  pos : vec2<f32>,
  vel : vec2<f32>,
  mass: f32,
  kind: f32,
  fptr: f32,
  bptr: f32,
};

struct SimParams {
  world_size: f32,
  dt : f32,
  well_depth : f32,
  attract_coeff : f32,
  repulse_coeff: f32,
  friction_coeff: f32,
  grid_size_side: f32,
};
@group(0) @binding(0) var<uniform> params : SimParams;
@group(0) @binding(1) var<storage, read> particlesSrc : array<Particle>;
@group(0) @binding(2) var<storage, read_write> particlesDst : array<Particle>;
@group(0) @binding(3) var<storage, read_write> bucket_indeces : array<i32>;

@compute
@workgroup_size(64)
fn main(@builtin(global_invocation_id) global_invocation_id: vec3<u32>) {
  let total = arrayLength(&particlesSrc);
  let index = global_invocation_id.x;
  if (index >= total) {
    return;
  }

  var vPos : vec2<f32> = particlesSrc[index].pos;

  var vVel : vec2<f32> = particlesSrc[index].vel;

  var bucket = compute_bucket(vPos);
  var newIndex: f32 = -1.0;

  var i : u32 = index;
  loop {
    if (i >= total ) {
      break;
    }
    if (i == index) {
      continue;
    }

    if(compute_bucket(particlesSrc[i].pos) == bucket) {
        newIndex = f32(i);
        particlesDst[i].bptr = f32(index);
        break;
    }
    continuing {
       i = i + 1u;
     }
  }

  //write back
  particlesDst[index].pos = vPos;
  particlesDst[index].vel = vVel;
  particlesDst[index].mass = particlesSrc[index].mass;
  particlesDst[index].kind = particlesSrc[index].kind;
  particlesDst[index].fptr = newIndex;

  //check if end
  if(particlesSrc[index].fptr == -1.0) {
   bucket_indeces[bucket] = i32(index);
  }
}


 fn compute_bucket(position: vec2<f32>) -> u32 {

     let num_grids_side: u32 = u32(params.world_size / params.grid_size_side);

    let x_bucket = u32(floor(position.x / params.grid_size_side));
    let y_bucket = u32(floor(position.y / params.grid_size_side));

    return y_bucket * num_grids_side + x_bucket;
 }
