struct Particle {
  pos : vec2<f32>,
  vel : vec2<f32>,
};

struct SimParams {
  dt : f32,
  G: f32
};

@group(0) @binding(0) var<uniform> params : SimParams;
@group(0) @binding(1) var<storage, read> particlesSrc : array<Particle>;
@group(0) @binding(2) var<storage, read_write> particlesDst : array<Particle>;

// https://github.com/austinEng/Project6-Vulkan-Flocking/blob/master/data/shaders/computeparticles/particle.comp
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

  var aAccum : vec2<f32> = vec2<f32>(0.0, 0.0);
  var dAccum : vec2<f32> = vec2<f32>(0.0, 0.0);


  var i : u32 = 0u;
  loop {
    if (i >= total) {
      break;
    }
    if (i == index) {
      continue;
    }

     let pos = particlesSrc[i].pos;
     
     let distance_vector: vec2<f32> = pos - vPos;
     
//     let vel = particlesSrc[i].vel;
     var distance = pow(distance_vector, vec2<f32>(2.0, 2.0));
     var distance_squared: f32 = distance.x + distance.y; 
     var dist = sqrt(distance_squared);
      if (dist < 0.009109375 ) {
         continue; 
    }
     var mag: f32 = params.G / distance_squared; //(distance_squared);
     var accel: vec2<f32> = (distance_vector / sqrt(distance_squared)) * mag;
    // var accel: vec2<f32> = mat2x2<f32>(0.0, -1.0, 1.0, 0.0) * accelm;
     aAccum = aAccum + accel;
     dAccum = dAccum + distance_vector / 50000.0;
     if(length(aAccum) < 0.0 ) {
        vVel = vec2<f32>(0.0, 0.0);
        break;
     }
     continuing {
       i = i + 1u;
     }
  }

  vVel =  sqrt(length(aAccum) * length(dAccum) ) * mat2x2<f32>(0.0, -1.0, 1.0, 0.0) * aAccum * params.dt; 
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

  // //vPos.x += params.dt / 100.0;
  // //Wrap around boundary
  // if (vPos.x < -1.0) {
  //   vPos.x = 1.0;
  // }
  // if (vPos.x > 1.0) {
  //   vPos.x = -1.0;
  // }
  // if (vPos.y < -1.0) {
  //   vPos.y = 1.0;
  // }
  // if (vPos.y > 1.0) {
  //   vPos.y = -1.0;
  // }

  // Write back
  particlesDst[index] = Particle(vPos, vVel);
}
