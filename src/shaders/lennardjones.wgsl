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


// https://github.com/austinEng/Project6-Vulkan-Flocking/blob/master/data/shaders/computeparticles/particle.comp
@compute
@workgroup_size(64)
fn main(@builtin(global_invocation_id) global_invocation_id: vec3<u32>) {
  let total = arrayLength(&particlesSrc);
  let max_types = u32(sqrt(f32(arrayLength(&attraction_matrix))));
  let index = global_invocation_id.x;
  if (index >= total) {
    return;
  }

  var vPos : vec2<f32> = particlesSrc[index].pos;
  var vVel : vec2<f32> = particlesSrc[index].vel;
  var vMass : f32 = particlesSrc[index].mass;
  var vKind : u32 =  u32(particlesSrc[index].kind * f32(max_types));

  var aAccum : vec2<f32> = vec2<f32>(0.0, 0.0);
  var cAccum : vec2<f32> = vec2<f32>(0.0, 0.0);
  var collided : u32 = 0u;

  var i : u32 = 0u;
  loop {
    if (i >= total) {
      break;
    }
    if (i == index) {
      continue;
    }

     let pos = particlesSrc[i].pos;
     let mass = particlesSrc[i].mass;
     let vel = particlesSrc[i].vel;
     let kind = u32(particlesSrc[i].kind * f32(max_types));
     let distance_vector: vec2<f32> = pos - vPos;

//     let vel = particlesSrc[i].vel;
     var distance = pow(distance_vector, vec2<f32>(2.0, 2.0));
     var distance_squared: f32 = distance.x + distance.y;
     var dist = sqrt(distance_squared);
     var col_length = 1.0; //(sqrt(mass) + sqrt(vMass)) / 2.0; //sigma
    // var well_depth = 500.0; //e
     var col_dist = (dist) / col_length;
     var z = (col_dist + 10.22462) / 10.0;

      var mag = 0.0;
     if(col_dist <= 1.0) {
        mag = params.repulse_coeff  * (params.well_depth * col_dist - params.well_depth);
     } else {
        var term_1 = pow(col_length, 6.0) / pow(z, 7.0);
        var mat_index = vKind * max_types + kind;

        mag = -1.0 * params.attract_coeff * params.well_depth * attraction_matrix[mat_index].elem * term_1 * (term_1 * z - 0.5); /// (distance_squared + 0.0000000000001);
    }

        var accel: vec2<f32> = (distance_vector / sqrt(distance_squared + 0.0000000000001)) * mag / vMass;
     aAccum = aAccum + accel;

     continuing {
       i = i + 1u;
     }
  }

  var nvVel = (vVel + (aAccum * params.dt)) * params.friction_coeff;
  // if(collided != 1u) {
  //     vVel = 0.98 * vVel;
  // }

  vPos = vPos + cAccum;
  vPos = vPos + (vVel + nvVel) / 2.0 * params.dt;

  vVel = nvVel;
//     vPos.x = vPos.x + 0.001;

//   if (cMassCount > 0) {
//     cMass = cMass * (1.0 / f32(cMassCount)) - vPos;
//   }
//   if (cVelCount > 0) {
//     cVel *= 1.0 / f32(cVelCount);
//   }

//   vVel = vVel + (cMass * params.rule1Scale) +
//       (colVel * params.rule2Scale) +
//       (cVel * params.rule3Scale);

//   // clamp velocity for a more pleasing simulation
//   vVel = normalize(vVel) * clamp(length(vVel), 0.0, 0.1);

//   // kinematic update
//   vPos += vVel * params.dt;

  //vPos.x += params.dt / 100.0;

//   let world_size: f32 = 1000.0;
//   // //Wrap around boundary
//   if (vPos.x < -1.0 * world_size) {
//     vPos.x = world_size;
//   }
//   if (vPos.x > world_size) {
//     vPos.x = -1.0 * world_size;
//   }
//   if (vPos.y < -1.0 * world_size) {
//     vPos.y = world_size;
//   }
//   if (vPos.y > world_size) {
//     vPos.y = -1.0 * world_size;
//   }


  // Write back
  particlesDst[index] = Particle(vPos, vVel, particlesSrc[index].mass, particlesSrc[index].kind, particlesSrc[index].fptr, particlesSrc[index].bptr);
}
