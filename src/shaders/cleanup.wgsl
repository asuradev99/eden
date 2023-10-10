struct Particle {
  pos : vec2<f32>,
  vel : vec2<f32>,
  mass: f32,
  kind: f32,
  fptr: f32,
  bptr: f32,
  debug: f32,
};


@group(0) @binding(0) var<storage, read> particlesSrc : array<Particle>;
@group(0) @binding(1) var<storage, read_write> particlesDst : array<Particle>;
@group(0) @binding(2) var<storage, read_write> bucket_indeces : array<i32>;

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
  var vMass: f32 = 1.0;

  // particlesDst[index] = Particle(vPos, vVel, vMass, particlesSrc[index].kind, -1.0,  -1.0);

    particlesDst[index] = Particle(vPos, vVel, vMass, particlesSrc[index].kind, particlesSrc[index].fptr, particlesSrc[index].bptr, particlesSrc[index].debug);

   if(index < arrayLength(&bucket_indeces)) {
       bucket_indeces[index] = -1;
  }

}
