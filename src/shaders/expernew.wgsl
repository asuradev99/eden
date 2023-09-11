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
  var vMass: f32 = particlesSrc[index].mass;
  var aAccum : vec2<f32> = vec2<f32>(0.0, 0.0);



  var vBucket: u32 = compute_bucket(vPos, vMass);

  var leaderIndex = particlesSrc[index].bptr;
  var bugFix: u32 = 0u;

  let num_grids_side: u32 = u32(vMass);


    for(var i = 0; i < 9; i++) {

         let grid_size_side: f32 = params.world_size / vMass;

         var x_bucket = i32(floor(vPos.x / grid_size_side)) + NEIGHBORHOOD[i].x;
         var y_bucket = i32(floor(vPos.y / grid_size_side)) + NEIGHBORHOOD[i].y;

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
              continue;
            }
            //if( particlesSrc[u32(particlesSrc[u32(nextptr)].bptr)].fptr != particlesSrc[u32(nextptr)].bptr ) {
            //    break;
            //}
            let accel = calculate_accel(index, u32(nextptr));
             aAccum = aAccum + grid_size_side * accel;
             continuing {
                  nextptr = i32(particlesSrc[u32(nextptr)].bptr);
             }
        }


    }


   // aAccum = normalize(aAccum) * clamp(length(aAccum), 0.0, 100.0 * params.grid_size_side / (params.dt));

  var nvVel = (vVel + (aAccum * params.dt)) * params.friction_coeff;

   // nvVel = normalize(nvVel) * clamp(length(nvVel), 0.0, params.grid_size_side / (params.dt * 20.0));

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
  particlesDst[index] = Particle(vPos, vVel, vMass, particlesSrc[index].kind, -1.0, -1.0);
}

fn calculate_accel(index: u32, i: u32 ) -> vec2<f32> {

     let max_types = u32(sqrt(f32(arrayLength(&attraction_matrix))));

     var vKind : u32 =  u32(particlesSrc[index].kind * f32(max_types));

     var vMass : f32 = particlesSrc[index].mass;

     let pos = particlesSrc[i].pos;
     let mass = 1.0; //particlesSrc[i].mass;
     let vel = particlesSrc[i].vel;
     let kind = u32(particlesSrc[i].kind * f32(max_types));
     let distance_vector: vec2<f32> = pos - particlesSrc[index].pos;

     var distance = pow(distance_vector, vec2<f32>(2.0, 2.0));
     var distance_squared: f32 = distance.x + distance.y;
     let grid_size_side: f32 = params.world_size / vMass;

     var dist = sqrt(distance_squared) / grid_size_side;

     var beta: f32 = 1.0 / grid_size_side;

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

     var accel: vec2<f32> = params.well_depth * (distance_vector / sqrt(distance_squared + 0.0000000000001)) * mag ;

     return accel;

}


 fn compute_bucket(position: vec2<f32>, vMass: f32) -> u32 {

    let num_grids_side: u32 = u32(vMass); // u32(params.world_size / params.grid_size_side);

    let grid_size_side: f32 = params.world_size / vMass;

    let x_bucket = u32(floor(position.x / grid_size_side));
    let y_bucket = u32(floor(position.y / grid_size_side));

    return y_bucket * num_grids_side + x_bucket;
 }
