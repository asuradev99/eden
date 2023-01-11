

struct Particle {
  pos : vec2<f32>,
  vel : vec2<f32>,
  mass: f32, 
  kind: f32
};

struct SimParams {
  dt : f32,
  well_depth : f32, 
  attract_coeff : f32, 
  repulse_coeff: f32, 
  friction_coeff: f32,
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
     var col_length = (sqrt(mass) + sqrt(vMass)) / 2.0;
     if (dist <= col_length) {
        vVel =  vVel - ((2.0 * mass / (mass + vMass)) * (dot(vVel - vel, vPos - pos) / (distance_squared + 0.0000000000001)) * -0.95 * distance_vector); 
         cAccum = cAccum + (-1.0 * distance_vector / dist) * (col_length - dist) / 2.0;
         collided = 1u;
         //vVel = vec2<f32>(1023981012.0, 19283912837.09);
         continue;
     }
     var mat_index = vKind * max_types + kind; 
     var mag: f32 = vMass * mass * attraction_matrix[mat_index].elem  / distance_squared; //(distance_squared);
     var accel: vec2<f32> = (distance_vector / sqrt(distance_squared + 0.0000000000001)) * mag / vMass;
    // var accel: vec2<f32> = mat2x2<f32>(0.0, -1.0, 1.0, 0.0) * accelm;
     aAccum = aAccum + accel;
     
     continuing {
       i = i + 1u;
     }
  }

  vVel = vVel + aAccum * params.dt; 
  // if(collided != 1u) {
  //     vVel = 0.98 * vVel; 
  // }

  vPos = vPos + cAccum;
  vPos = vPos + vVel * params.dt;
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
  particlesDst[index] = Particle(vPos, vVel, particlesSrc[index].mass, particlesSrc[index].kind);
}
