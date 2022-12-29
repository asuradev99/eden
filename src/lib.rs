use rand::prelude::*;


#[derive(Clone, Copy, Debug)]
pub struct Camera {
    pub x: f32,
    pub y: f32,
    pub zoom: f32,
    pub aspect_ratio: f32,
}

pub const CIRCLE_RES: u32 = 128;
pub const DEFAULT_COMPUTE_SHADER: &str = include_str!("shaders/aesthetic.wgsl");

impl Camera {
    pub fn new(zoom: f32, aspect_ratio: f32) -> Self {
        Camera {
            x: 0.0,
            y: 0.0,
            zoom: zoom,
            aspect_ratio: aspect_ratio,
        }
    }

    pub fn to_slice(&self) -> [f32; 4] {
        [self.x, self.y, self.zoom, self.aspect_ratio]
    }
}

#[derive(Clone, Debug)]

pub struct Params {
    pub attraction_matrix: Vec<f32>,
    pub dt: f32,
    pub num_particles: u32,
    pub world_size: f32,
    pub shader_buffer: String,
}


impl Params {
    pub fn new() -> Self {
        Params {
            attraction_matrix: vec![2.0, 0.0, 0.0, 0.0],
            dt: 0.001,
            num_particles: 20000,
            shader_buffer: DEFAULT_COMPUTE_SHADER.to_string(),
            world_size: 4.0,
        }
    }
    pub fn to_slice(&self) -> [f32; 1] {
        [self.dt]
    }

    pub fn attraction_matrix_slice(&self) -> &[f32] {
        self.attraction_matrix.as_slice()
    }
}

// const PARAMS: [f32; 2] = [
//     0.001, //dt
//     0.01//Gravitational constant
// ];

/// Example struct holds references to wgpu resources and frame persistent data
/// 
/// 
#[derive(Clone, Copy, Debug)]

pub struct Particle {
    pos: (f32, f32),
    vel: (f32, f32),
    mass: f32, 
    kind: f32
}

impl Particle {
    pub fn to_slice(&self) -> [f32; 6] {
        [self.pos.0, self.pos.1, self.vel.0, self.vel.1, self.mass, self.kind] 
    }
    pub fn new_random(params: &Params) -> Self {
        let mut rng = rand::thread_rng();
        let mut unif = || (rng.gen::<f32>() * 2f32 - 1f32) * params.world_size;
        let max_types: f32 = f32::sqrt(params.attraction_matrix.len() as f32 / 4.0); 
        let mut rng = rand::thread_rng();

        Self {
            pos: (unif(), unif() / 3.0),
            vel: (0.0, 0.0),
            mass: 0.001, 
            kind: (rng.gen_range(0..max_types as u32) as f32) / (max_types as f32) ,
        }
    }
    pub fn new() -> Self {
        Self{
            pos: (0.0, 0.0),
            vel: (0.0, 0.0),
            mass: 100.0, 
            kind: 0.01,
        }
    }

}

pub fn generate_circle(radius: f32) -> [f32; (CIRCLE_RES * 2)  as usize ] {
    use std::f64::consts::PI;
    use std::convert::TryInto;
    let mut coords = Vec::<f32>::new();
    for i in 0..CIRCLE_RES {
        coords.push(radius * ((2.0 * PI * i as f64 / CIRCLE_RES as f64) as f32).cos());
        coords.push(radius * ((2.0 * PI * i as f64 / CIRCLE_RES as f64) as f32).sin());
    }
    coords.try_into().unwrap_or_else(|v: Vec<f32>| panic!("Expected a Vec of length {} but it was {}", CIRCLE_RES, v.len()))
}
