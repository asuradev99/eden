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

  var cVel : vec2<f32> = vec2<f32>(0.0, 0.0);

 // var i : u32 = 0u;
//   loop {
//     if (i >= total) {
//       break;
//     }
//     if (i == index) {
//       continue;
//     }

//     let pos = particlesSrc[i].pos;
//     let vel = particlesSrc[i].vel;

    
//     continuing {
//       i = i + 1u;
//     }
// //   }
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

  vPos.x += 0.001;
  // Wrap around boundary
  if (vPos.x < -1.0) {
    vPos.x = 1.0;
  }
  if (vPos.x > 1.0) {
    vPos.x = -1.0;
  }
  if (vPos.y < -1.0) {
    vPos.y = 1.0;
  }
  if (vPos.y > 1.0) {
    vPos.y = -1.0;
  }

  // Write back
  particlesDst[index] = Particle(vPos, vVel);
}
