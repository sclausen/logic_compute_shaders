use std::{
    ops::{Add, AddAssign, Div, Mul, Sub},
    sync::{atomic::AtomicU32, Arc, Mutex},
};

use rayon::prelude::*;

use bevy::{ecs::system::Resource, math::Vec2, render::render_resource::ShaderType};

use crate::{particle::Particle, particle_config::ParticleConfig};

pub const HASH_K1: u32 = 15823;
pub const HASH_K2: u32 = 9737333;

const OFFSETS: &[(i32, i32)] = &[
    (-1, 1),
    (0, 1),
    (1, 1),
    (-1, 0),
    (0, 0),
    (1, 0),
    (-1, -1),
    (0, -1),
    (1, -1),
];

#[derive(Debug, Copy, Clone, ShaderType, bytemuck::Zeroable, bytemuck::Pod)]
#[repr(C)]
pub struct Entry {
    original_index: u32,
    hash: u32,
    key: u32,
}

#[derive(Resource)]
pub struct ParticleSimulation {
    pub cell_offsets: Vec<Point<u32>>,
    pub radius: f32,
    pub start_indices: Vec<u32>,
    pub spatial_lookup: Vec<Entry>,
    pub particles: Vec<Particle>,
    pub particle_config: ParticleConfig,
    pub spatial_offsets: Vec<u32>,
}

impl ParticleSimulation {
    fn simulation_step(&mut self) {
        for particle in self.particles.iter() {
            self.foreach_point_within_radius(particle);
        }
    }

    fn foreach_point_within_radius(&self, particle: &Particle) {
        let centre = Self::position_to_cell_coord(particle.position, self.radius);
        let sqr_radius = self.radius * self.radius;

        for offset in self.cell_offsets.iter() {
            let key: u32 = self.get_key_from_hash(Self::hash_cell((*offset + centre).into()));
            let cell_start_index = self.start_indices[key as usize];

            for i in cell_start_index..self.spatial_lookup.len() as u32 {
                if self.spatial_lookup[i as usize].key != key {
                    break;
                }

                let particle_index = self.spatial_lookup[i as usize].original_index as usize;
                let sqr_distance = particle
                    .position
                    .distance_squared(self.particles[particle_index].position);

                if sqr_distance < sqr_radius {
                    self.process_particle(&self.particles[particle_index]);
                }
            }
        }
    }

    fn update_spatial_lookup(&mut self, particles: Vec<Particle>, radius: f32) {
        self.particles = particles;
        self.radius = radius;

        let spatial_lookup = Arc::new(Mutex::new(vec![
            Entry {
                original_index: 0,
                hash: 0,
                key: 0,
            };
            self.particles.len()
        ]));

        self.particles
            .par_iter()
            .enumerate()
            .for_each(|(i, particle)| {
                let point = Self::position_to_cell_coord(particle.position, self.radius);
                let hash = Self::hash_cell(point);
                let key = self.get_key_from_hash(hash);

                let spatial_lookup = Arc::clone(&spatial_lookup);
                let mut spatial_lookup = spatial_lookup.lock().unwrap();
                spatial_lookup[i] = Entry {
                    original_index: i as u32,
                    hash,
                    key,
                };
            });

        let mut spatial_lookup = spatial_lookup.lock().unwrap();
        spatial_lookup.sort_by(|a, b| a.key.cmp(&b.key));

        self.spatial_lookup = spatial_lookup.clone();

        let start_indices: Vec<std::sync::atomic::AtomicU32> =
            Vec::with_capacity(self.cell_offsets.len());

        self.cell_offsets.par_iter().enumerate().for_each(|(i, _)| {
            let key: u32 = self.spatial_lookup[i].key;
            let key_prev = if i == 0 {
                u32::MAX
            } else {
                self.spatial_lookup[i - 1].key
            };
            if key != key_prev {
                start_indices[key as usize].store(i as u32, std::sync::atomic::Ordering::SeqCst);
            }
        });

        self.start_indices = start_indices
            .iter()
            .map(|x| x.load(std::sync::atomic::Ordering::SeqCst))
            .collect();
    }

    fn hash_cell(offset: Point<i32>) -> u32 {
        let cell_u: Point<u32> = Point(offset.0 as u32, offset.1 as u32);
        let a: u32 = cell_u.0 * HASH_K1;
        let b: u32 = cell_u.1 * HASH_K2;
        return a + b;
    }

    fn get_key_from_hash(&self, hash: u32) -> u32 {
        return hash % self.cell_offsets.len() as u32;
    }

    fn position_to_cell_coord(position: Vec2, radius: f32) -> Point<i32> {
        let x = (position.x / radius).floor() as i32;
        let y = (position.y / radius).floor() as i32;
        Point(x, y)
    }

    fn process_particle(&self, particle: &Particle) {
        let point = Self::position_to_cell_coord(particle.position, self.radius);
        let key = self.get_key_from_hash(Self::hash_cell(point));

        let mut total_force: Point<f32> = Point(0.0, 0.0);
        let origin_cell =
            Self::position_to_cell_coord(particle.position, self.particle_config.r_max);
        let sqr_radius: f32 = self.particle_config.r_max * self.particle_config.r_max;
        let velocity = particle.velocity;

        for i in 0..=9 {
            let offset: Point<i32> = OFFSETS[i].into_point();

            let hash = Self::hash_cell(origin_cell.clone() + offset);
            let key = self.get_key_from_hash(hash);
            let curr_index = self.spatial_offsets[key as usize];
            let entry = self.spatial_lookup[curr_index as usize];

            if entry.key != key {
                break;
            } // Exit if no longer looking at the correct bin
            if entry.hash != hash {
                continue;
            } // Skip if hash does not match
            let neighbour_index = entry.original_index as usize;
            // if neighbour_index == id {
            //     continue;
            // } // Skip if looking at self
            let neighbor = self.particles[neighbour_index];
            let offset_to_neighbour = neighbor.position - particle.position;
            let sqr_dst_to_neighbour = offset_to_neighbour.length_squared();

            if sqr_dst_to_neighbour > sqr_radius {
                continue;
            } // Skip if not within radius

            let r = sqr_dst_to_neighbour.sqrt();
            let a = self.particle_config.attraction_matrix[particle.particle_type as usize
                * self.particle_config.m as usize
                + neighbor.particle_type as usize];

            if r > 0.0 && r < self.particle_config.r_max {
                let f = Self::force(r / self.particle_config.r_max, a);
                total_force += offset_to_neighbour.into_point() / r
                    * f
                    * self.particle_config.r_max
                    * self.particle_config.force_factor;
            }

            let neighbour_velocity = self.particles[neighbour_index].velocity;
            total_force += neighbour_velocity.into_point() - velocity;
        }

        //     while curr_index < particle_config.n {
        //         let index_data = spatial_lookup[curr_index];
        //         curr_index++;
        //         if index_data.z != key {
        //             break;
        //         } // Exit if no longer looking at the correct bin
        //         if index_data.y != hash {
        //             continue;
        //         } // Skip if hash does not match

        //         let neighbour_index = index_data.x;
        //         if neighbour_index == id {
        //             continue;
        //         } // Skip if looking at self

        //         let neighbour = particles[neighbour_index];
        //         let offset_to_neighbour = neighbour.position - particles[id].position;
        //         let sqr_dst_to_neighbour = dot(offset_to_neighbour, offset_to_neighbour);

        //         if sqr_dst_to_neighbour > sqr_radius {
        //             continue;
        //         } // Skip if not within radius

        //         let r = sqrt(sqr_dst_to_neighbour);
        //         let a = attraction_matrix[particles[id].particle_type * particle_config.m + neighbour.particle_type];

        //         if r > 0.0 && r < particle_config.r_max {
        //             let f = force(r / particle_config.r_max, a);
        //             total_force += offset_to_neighbour / r * f * particle_config.r_max * particle_config.force_factor;
        //         }

        //         //let f = force(dst / particle_config.r_max, a);
        //         //total_force += vec2<f32>(rx / dst * f, ry / dst * f) * particle_config.r_max * particle_config.force_factor;

        //         //let neighbour_velocity = particles[neighbour_index].velocity;
        //         //total_force += neighbour_velocity - velocity;
        //     }
        // }

        // //particles[id].velocity = (particles[id].velocity + total_force * particle_config.dt) * particle_config.friction_factor * delta_time;
        // particles[id].velocity = (particles[id].velocity + total_force * particle_config.dt) * particle_config.friction_factor ;
    }

    fn force(r: f32, a: f32) -> f32 {
        let beta: f32 = 0.3;
        if r < beta {
            return r / beta - 1.0;
        } else if beta < r && r < 1.0 {
            return a * (1.0 - (2.0 * r - 1.0 - beta).abs() / (1.0 - beta));
        } else {
            return 0.0;
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Point<T>(pub T, pub T)
where
    T: Add<Output = T> + Copy;

impl<T> Add for Point<T>
where
    T: Add<Output = T> + Copy,
{
    type Output = Self;

    fn add(self, other: Self) -> Self::Output {
        Point(self.0 + other.0, self.1 + other.1)
    }
}

impl AddAssign for Point<f32> {
    fn add_assign(&mut self, other: Self) {
        self.0 += other.0;
        self.1 += other.1;
    }
}

impl Add<Point<i32>> for Point<u32> {
    type Output = Point<i32>;

    fn add(self, other: Point<i32>) -> Self::Output {
        Point(self.0 as i32 + other.0, self.1 as i32 + other.1)
    }
}

impl Sub<Vec2> for Point<f32> {
    type Output = Self;

    fn sub(self, rhs: Vec2) -> Self::Output {
        Point(self.0 - rhs.x, self.1 - rhs.y)
    }
}

impl Div<f32> for Point<f32> {
    type Output = Self;

    fn div(self, rhs: f32) -> Self::Output {
        Point(self.0 / rhs, self.1 / rhs)
    }
}

impl Mul<f32> for Point<f32> {
    type Output = Self;

    fn mul(self, rhs: f32) -> Self::Output {
        Point(self.0 * rhs, self.1 * rhs)
    }
}

pub trait IntoPoint {
    type Output;
    fn into_point(self) -> Self::Output;
}

impl<T> IntoPoint for (T, T)
where
    T: Add<Output = T> + Copy,
{
    type Output = Point<T>;

    fn into_point(self) -> Self::Output {
        Point(self.0, self.1)
    }
}

impl IntoPoint for Vec2 {
    type Output = Point<f32>;

    fn into_point(self) -> Self::Output {
        Point(self.x, self.y)
    }
}

impl TryFrom<(i32, i32)> for Point<u32> {
    type Error = std::num::TryFromIntError;

    fn try_from(value: (i32, i32)) -> Result<Self, Self::Error> {
        Ok(Point(value.0 as u32, value.1 as u32))
    }
}

impl From<(i32, i32)> for Point<i32> {
    fn from(value: (i32, i32)) -> Self {
        Point(value.0, value.1)
    }
}
