use bevy::{
    log,
    prelude::*,
    render::{extract_resource::ExtractResource, render_resource::ShaderType},
};
use nanorand::{Rng, WyRand};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Resource, Reflect, Serialize, Deserialize, ExtractResource)]
#[reflect(Resource)]
pub struct ParticleConfig {
    pub n: usize,
    pub dt: f32,
    pub friction_half_life: f32,
    pub r_max: f32,
    pub variants: usize,
    pub force_factor: f32,
    pub friction_factor: f32,
    pub attraction_matrix: Vec<f32>,
    pub is_grayscale: bool,
    pub world_width: u32,
    pub world_height: u32,
    pub seed: u64,
    #[reflect(ignore)]
    #[serde(skip)]
    pub rng: WyRand,
}

impl ParticleConfig {
    pub fn extract_shader_variables(&self) -> (ShaderParticleConfig, Vec<f32>) {
        let shader_config = ShaderParticleConfig {
            n: self.n as u32,
            dt: self.dt,
            friction_half_life: self.friction_half_life,
            r_max: self.r_max,
            variants: self.variants as u32,
            force_factor: self.force_factor,
            friction_factor: self.friction_factor,
            world_width: self.world_width as f32,
            world_height: self.world_height as f32,
        };
        (shader_config, self.attraction_matrix.clone())
    }

    pub fn calculate_world_size(r_max: f32) -> (u32, u32) {
        let desired_size: f32 = 800.0;
        let cell_size = r_max;

        let num_cells_w = (desired_size / cell_size).round();
        let num_cells_h = num_cells_w;

        let new_width = (num_cells_w * cell_size).round() as u32;
        let new_height = (num_cells_h * cell_size).round() as u32;

        log::debug!("World size: {} x {}", new_width, new_height);

        (new_width, new_height)
    }
}

#[derive(Debug, Clone, ShaderType)]
pub struct ShaderParticleConfig {
    pub n: u32,
    pub dt: f32,
    pub friction_half_life: f32,
    pub r_max: f32,
    pub variants: u32,
    pub force_factor: f32,
    pub friction_factor: f32,
    pub world_width: f32,
    pub world_height: f32,
}

impl Default for ParticleConfig {
    fn default() -> Self {
        let friction_half_life = 0.02;
        let dt = 0.002;
        let variants = 6;
        let n = 1;
        let r_max = 50.0;

        let (world_width, world_height) = Self::calculate_world_size(r_max);

        let mut rng = WyRand::default();

        Self {
            n,
            dt,
            friction_half_life,
            r_max,
            variants,
            force_factor: 15.0,
            friction_factor: 0.5f32.powf(dt / friction_half_life),
            attraction_matrix: make_random_matrix(variants, &mut rng),
            is_grayscale: true,
            world_width,
            world_height,
            seed: 0,
            rng,
        }
    }
}

pub fn make_random_matrix(variants: usize, rng: &mut WyRand) -> Vec<f32> {
    let mut matrix = vec![0.0; variants * variants];
    for i in 0..variants {
        for j in 0..variants {
            matrix[i * variants + j] = rng.generate::<f32>() * 2f32 - 1f32;
        }
    }
    matrix
}
