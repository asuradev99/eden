


struct Particle {
  pos : vec2<f32>,
  vel : vec2<f32>,
  mass: f32,
  kind: f32,
  fptr: f32,
  bptr: f32
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

struct AttractionMatrixEntry {
  elem: f32,
  _pad1: f32,
  _pad2: f32,
  _pad3: f32,
}
@group(0) @binding(0) var<uniform> params : SimParams;
@group(0) @binding(1) var<storage, read> particlesSrc : array<Particle>;
@group(0) @binding(2) var<storage, read_write> particlesDst : array<Particle>;
@group(0) @binding(3) var<storage, read> attraction_matrix : array<AttractionMatrixEntry>;
@group(0) @binding(4) var<storage, read_write> bucket_indeces : array<i32>;


@compute
@workgroup_size(64)
fn main(@builtin(global_invocation_id) global_invocation_id: vec3<u32>) {
var NEIGHBORHOOD = array(
   vec2<i32>(-1, -1),
    vec2<i32>(0, -1),
 vec2<i32>(1, -1),
    vec2<i32>(-1, 0),
    vec2<i32>(0, 0),
   vec2<i32>(1, 0),
    vec2<i32>(-1, 1),
    vec2<i32>(0, 1),
    vec2<i32>(1, 1)
 );


  let total = arrayLength(&particlesSrc);
  let index = global_invocation_id.x;
  if (index >= total) {
    return;
  }

  var vPos : vec2<f32> = particlesSrc[index].pos;
  var vVel : vec2<f32> = particlesSrc[index].vel;
  var vMass: f32 = 0.0; // particlesSrc[index].mass;
  var aAccum : vec2<f32> = vec2<f32>(0.0, 0.0);



  var vBucket: u32 = compute_bucket(vPos);

  var leaderIndex = particlesSrc[index].bptr;
  var bugFix: u32 = 0u;

  let num_grids_side: u32 = u32(params.world_size / params.grid_size_side);
  if(leaderIndex > -1.0) {

    var testBucket: u32 = compute_bucket(particlesSrc[u32(leaderIndex)].pos);
    if(vBucket != testBucket) {
        bugFix = 1u;
    }
  }

    for(var i = 0; i < 9; i++) {
         var x_bucket = i32(floor(vPos.x / params.grid_size_side)) + NEIGHBORHOOD[i].x;
         var y_bucket = i32(floor(vPos.y / params.grid_size_side)) + NEIGHBORHOOD[i].y;

        if(x_bucket < 0 || x_bucket >= i32(num_grids_side) || y_bucket < 0 || y_bucket >= i32(num_grids_side)) {
            continue;
        }

       // vMass += 1.0;


        var newBucket: i32 = y_bucket * i32(num_grids_side) + x_bucket;
        var nextptr : i32 = bucket_indeces[u32(newBucket)];

         loop {
            if (nextptr == -1) {
              break;
            }
            if (nextptr == i32(index)) {
              if(bugFix == 1u) {
                break;
              }
              continue;
            }
            let accel = calculate_accel(index, u32(nextptr));
             aAccum = aAccum + params.grid_size_side * accel;
             continuing {
                  nextptr = i32(particlesSrc[u32(nextptr)].bptr);
             }
        }


    }

  var nvVel = (vVel + (aAccum * params.dt)) * params.friction_coeff;


  vPos = vPos + (vVel + nvVel) / 2.0 * params.dt;

  let fudge = 0.00001;

   if (vPos.x < fudge ) {
     vPos.x = fudge; //params.world_size - params.world_size / fudge;
     nvVel.x = -1.0 * nvVel.x;

   }
   if (vPos.x > params.world_size - fudge) {
     vPos.x = params.world_size - fudge; // / fudge;
     nvVel.x = -1.0 * nvVel.x;
    }
   if (vPos.y < fudge) {
     vPos.y = fudge; //params.world_size - params.world_size / fudge;
     nvVel.y = -1.0 * nvVel.y;
   }
   if (vPos.y > params.world_size - fudge) {
     vPos.y = params.world_size - fudge; // / fudge;
     nvVel.y = -1.0 * nvVel.y;
   }
  vVel = nvVel;
  // Write back
  particlesDst[index] = Particle(vPos, vVel, vMass, particlesSrc[index].kind, particlesSrc[index].fptr, particlesSrc[index].bptr);
}

fn calculate_accel(index: u32, i: u32 ) -> vec2<f32> {

     let max_types = u32(sqrt(f32(arrayLength(&attraction_matrix))));

     var vKind : u32 =  u32(particlesSrc[index].kind * f32(max_types));

     var vMassTest : f32 = 1.0; //particlesSrc[index].mass;

     let pos = particlesSrc[i].pos;
     let mass = 1.0; //particlesSrc[i].mass;
     let vel = particlesSrc[i].vel;
     let kind = u32(particlesSrc[i].kind * f32(max_types));
     let distance_vector: vec2<f32> = pos - particlesSrc[index].pos;

     var distance = pow(distance_vector, vec2<f32>(2.0, 2.0));
     var distance_squared: f32 = distance.x + distance.y;
     var dist = sqrt(distance_squared) / params.grid_size_side;

     var beta: f32 = 1.0 / params.grid_size_side;

      var mag = 0.0;

     if(dist < beta) {
        mag = dist / beta - 1.0;
     } else if (dist > beta && dist < 1.0) {
         var mat_index = vKind * max_types + kind;
         mag = attraction_matrix[mat_index].elem * (1.0 - (abs((2.0 * dist) - 1.0 - beta) / (1.0 - beta)));
     } else {
         mag = 0.0;
         return vec2(0.0, 0.0);
     }

     var accel: vec2<f32> = params.well_depth * (distance_vector / sqrt(distance_squared + 0.0000000000001)) * mag / vMassTest;

     return accel;

}

fn compute_bucket(position: vec2<f32>) -> u32 {

     let num_grids_side: u32 = u32(params.world_size / params.grid_size_side);

    let x_bucket = u32(floor(position.x / params.grid_size_side));
    let y_bucket = u32(floor(position.y / params.grid_size_side));

    return y_bucket * num_grids_side + x_bucket;
}
