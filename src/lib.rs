use eframe::epaint::CircleShape;

use rand::prelude::*;

#[derive(Clone, Copy, Debug)]
pub struct Camera {
    pub x: f32,
    pub y: f32,
    pub zoom: f32,
    pub aspect_ratio: f32,
}

pub const SAMPLE_COUNT: u32 = 4;
pub const TEXTURE_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Bgra8Unorm; // wgpu::TextureFormat::Rgba8UnormSrgb;
pub const CIRCLE_RES: u32 = 16;
pub const DEFAULT_COMPUTE_SHADER: &str = include_str!("shaders/experimental.wgsl");

impl Camera {
    pub fn new(x: f32, y: f32, zoom: f32, aspect_ratio: f32) -> Self {
        Camera {
            x: x,
            y: y,
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
    pub well_depth: f32,
    pub attract_coeff: f32,
    pub repulse_coeff: f32,
    pub friction_coeff: f32,
    pub num_types: u32,
    pub num_grids_side: u32,
    pub play: bool,
    pub particle_radius: f32,
}

impl Params {
    pub fn new() -> Self {
        let mut attraction_matrix: Vec<f32> = Vec::new();
        let mut rng = rand::thread_rng();
        let mut unif = || (rng.gen::<f32>() * 2f32 - 1f32);
        let num_types: u32 = 1;
        for i in 0..num_types.pow(2) {
            attraction_matrix.extend_from_slice(&[unif(), 0.0, 0.0, 0.0]);
        }

        println!("{:?}", attraction_matrix);
        Params {
            num_types: num_types,
            attraction_matrix: attraction_matrix,
            dt: 0.001, //0.001,
            num_particles: 1000,
            shader_buffer: DEFAULT_COMPUTE_SHADER.to_string(),
            world_size: 50.0,
            well_depth: 30.0,
            attract_coeff: 1.0,
            repulse_coeff: 1.0,
            friction_coeff: 0.9,
            num_grids_side: 1,
            play: true,
            particle_radius: 1.0,
        }
    }

    pub fn randomize_matrix(&mut self) {
        let mut attraction_matrix: Vec<f32> = Vec::new();
        let mut rng = rand::thread_rng();
        let mut unif = || (rng.gen::<f32>() * 2f32 - 1f32);

        for i in 0..self.num_types.pow(2) {
            attraction_matrix.extend_from_slice(&[unif(), 0.0, 0.0, 0.0]);
        }

        self.attraction_matrix = attraction_matrix;
    }

    pub fn to_slice(&self) -> [f32; 7] {
        [
            self.world_size,
            self.dt,
            self.well_depth,
            self.attract_coeff,
            self.repulse_coeff,
            self.friction_coeff,
            (self.world_size / self.num_grids_side as f32),
        ]
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
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct Particle {
    pos: (f32, f32),
    vel: (f32, f32),
    pub mass: f32,
    kind: f32,
    pub fptr: f32,
    pub bptr: f32,
}

impl Particle {
    pub fn to_slice(&self) -> [f32; 8] {
        [
            self.pos.0, self.pos.1, self.vel.0, self.vel.1, self.mass, self.kind, self.fptr,
            self.bptr,
        ]
    }
    pub fn new_random(params: &Params) -> Self {
        let mut rng = rand::thread_rng();
        let mut unif = || (rng.gen::<f32>()) * params.world_size;
        let max_types: f32 = f32::sqrt(params.attraction_matrix.len() as f32 / 4.0);
        let mut rng = rand::thread_rng();

        Self {
            pos: (unif(), unif()),
            vel: (0.0, 0.0),
            mass: 1.0,
            kind: (rng.gen_range(0..max_types as u32) as f32) / (max_types as f32),
            bptr: -1.0,
            fptr: -1.0,
        }
    }
    pub fn new() -> Self {
        Self {
            pos: (0.0, 0.0),
            vel: (0.0, 0.0),
            mass: 100.0,
            kind: 0.01,
            fptr: -1.0,
            bptr: -1.0,
        }
    }
}

pub fn generate_circle(radius: f32) -> [f32; (CIRCLE_RES * 8) as usize] {
    use std::convert::TryInto;
    use std::f64::consts::PI;
    let mut coords = Vec::<f32>::new();

    for i in 0..(CIRCLE_RES) {
        coords.push(radius * ((2.0 * PI * i as f64 / CIRCLE_RES as f64) as f32).cos());
        coords.push(radius * ((2.0 * PI * i as f64 / CIRCLE_RES as f64) as f32).sin());

        coords.push(0.0);
        coords.push(0.0);

        coords.push(radius * ((2.0 * PI * (i + 1) as f64 / CIRCLE_RES as f64) as f32).cos());
        coords.push(radius * ((2.0 * PI * (i + 1) as f64 / CIRCLE_RES as f64) as f32).sin());
    }

    for i in 0..CIRCLE_RES {
        coords.push(0.01);
        coords.push(0.0)
    }

    coords.try_into().unwrap_or_else(|v: Vec<f32>| {
        panic!(
            "Expected a Vec of length {} but it was {}",
            CIRCLE_RES,
            v.len()
        )
    })
}
