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
};
@group(0) @binding(0) var<uniform> params : SimParams;
@group(0) @binding(1) var<storage, read> particlesSrc : array<Particle>;
@group(0) @binding(2) var<storage, read_write> particlesDst : array<Particle>;

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

  vPos.x += 0.01;

  //write back

  particlesDst[index] = Particle(vPos, vVel, particlesSrc[index].mass, particlesSrc[index].kind, particlesSrc[index].fptr, particlesSrc[index].bptr);
}


fn compute_bucket(position: vec2<f32>) {

}
