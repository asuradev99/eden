
use rand::prelude::*;

use crate::sim::DEFAULT_COMPUTE_SHADER;

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
    pub type_colors: Vec<f32>,
    pub play: bool,

}


impl Params {
    pub fn new() -> Self {

        let mut attraction_matrix: Vec<f32> = Vec::new();
        let num_types: u32 = 3;
        let mut type_colors: Vec<f32> = Vec::new();
        let mut rng = rand::thread_rng();
        let mut unif = || (rng.gen::<f32>());
        for _i in 0..(num_types * 3) {
            type_colors.extend_from_slice(&[unif(), 0.0, 0.0, 0.0]);
        } 
        let mut rng = rand::thread_rng();
        let mut unif = || (rng.gen::<f32>() * 2f32 - 1f32);
        for _i in 0..num_types.pow(2) {
            attraction_matrix.extend_from_slice(&[unif(), 0.0, 0.0, 0.0]);
        }

        Params {
            num_types: num_types,
            type_colors: type_colors,
            attraction_matrix: attraction_matrix,
            dt: 0.001,
            num_particles: 10,
            shader_buffer: DEFAULT_COMPUTE_SHADER.to_string(),
            world_size: 10.0,
            well_depth: 50000.0,
            attract_coeff: 1.0,
            repulse_coeff: 1.0,
            friction_coeff: 0.9,
            play: true,
        }
    }

    pub fn randomize_matrix(&mut self) {
        let mut attraction_matrix: Vec<f32> = Vec::new();
        let mut type_colors: Vec<f32> = Vec::new();
        let mut rng = rand::thread_rng();

        let mut unif = || (rng.gen::<f32>());
        for _i in 0..(self.num_types * 3) {
            type_colors.extend_from_slice(&[unif(), 0.0, 0.0, 0.0]);
        } 


        let mut unif = || (rng.gen::<f32>() * 2f32 - 1f32);

        for _i in 0..self.num_types.pow(2) {
            attraction_matrix.extend_from_slice(&[unif(), 0.0, 0.0, 0.0]);
        }

        self.attraction_matrix = attraction_matrix;
        self.type_colors = type_colors;
    }

    pub fn to_slice(&self) -> [f32; 5] {
        [self.dt, self.well_depth, self.attract_coeff, self.repulse_coeff, self.friction_coeff]
    }

    pub fn attraction_matrix_slice(&self) -> &[f32] {
        self.attraction_matrix.as_slice()
    }
}

