#import "particle.wgsl"::{Particle,ParticleConfig}
#import "gpu_sort.wgsl"::{SpatialIndex}
#import "utils.wgsl"::{force, get_cell, key_from_hash, hash_cell, get_offset, get_wrapped_neighbor_cell}

@group(0) @binding(0)
var<storage, read_write> particles: array<Particle>;

@group(0) @binding(1)
var<uniform> particle_config: ParticleConfig;

@group(0) @binding(2)
var<storage, read> attraction_matrix: array<f32>;

@group(0) @binding(3)
var<uniform> delta_time: f32;

@group(0) @binding(4)
var<storage, read_write> spatial_indices: array<SpatialIndex>;

@group(0) @binding(5)
var<storage, read_write> spatial_offsets: array<u32>;

const WORKGROUP_SIZE: u32 = #{WORKGROUP_SIZE};


fn id(invocation_id: vec3<u32>, num_workgroups: vec3<u32>) -> u32 {
    return invocation_id.y * u32(32) + invocation_id.x;
}

@compute @workgroup_size(WORKGROUP_SIZE, 1, 1)
fn update_velocities(
    @builtin(global_invocation_id) invocation_id: vec3<u32>,
    @builtin(num_workgroups) num_workgroups: vec3<u32>
) {
    let id = id(invocation_id, num_workgroups);

    var total_force: vec2<f32> = vec2<f32>(0.0, 0.0);
    let origin_cell = get_cell(particles[id].position, particle_config.r_max);
    let sqr_radius = particle_config.r_max * particle_config.r_max;

    for (var i: i32 = 0; i < 9; i++) {
        let offset = get_offset(i);
        let neighbor_cell_temp = origin_cell + offset;

        let neighbor_cell = vec2<i32>(
            (neighbor_cell_temp.x + i32(particle_config.world_width / particle_config.r_max)) % i32(particle_config.world_width / particle_config.r_max),
            (neighbor_cell_temp.y + i32(particle_config.world_height / particle_config.r_max)) % i32(particle_config.world_height / particle_config.r_max)
        );

        let hash = hash_cell(neighbor_cell);
        let key = key_from_hash(hash, particle_config.n);

        var curr_particle_index = spatial_offsets[key];

        while curr_particle_index < particle_config.n {
            let index_data = spatial_indices[curr_particle_index];
            curr_particle_index++;
            if index_data.key != key {
                break;
            } // Exit if no longer looking at the correct bucket
            if index_data.hash != hash {
                continue;
            } // Skip if hash does not match

            let neighbour_index = index_data.original_index;
            if neighbour_index == id {
                continue;
            } // Skip if looking at self

            let neighbour = particles[neighbour_index];

            var offset_to_neighbor = neighbour.position - particles[id].position;

             // Adjust position for wrapped distance
            for (var dim: i32 = 0; dim < 2; dim++) {
                if abs(offset_to_neighbor[dim]) > particle_config.world_width * 0.5 {
                    offset_to_neighbor[dim] -= sign(offset_to_neighbor[dim]) * particle_config.world_width;
                }
                if abs(offset_to_neighbor[dim + 1]) > particle_config.world_height * 0.5 {
                    offset_to_neighbor[dim + 1] -= sign(offset_to_neighbor[dim + 1]) * particle_config.world_height;
                }
            }

            let sqr_dst_to_neighbour = dot(offset_to_neighbor, offset_to_neighbor);

            if sqr_dst_to_neighbour > sqr_radius {
                continue;
            } // Skip if not within radius

            let r = sqrt(sqr_dst_to_neighbour);
            let a = attraction_matrix[particles[id].particle_type * particle_config.variants + neighbour.particle_type];

            if r > 0.0 && r < particle_config.r_max {
                let f = force(r / particle_config.r_max, a);
                total_force += offset_to_neighbor / r * f * particle_config.r_max * particle_config.force_factor;
            }
        }
    }

    particles[id].velocity = (particles[id].velocity + total_force * particle_config.dt) * particle_config.friction_factor;
}

@compute @workgroup_size(WORKGROUP_SIZE, 1, 1)
fn update_positions(
    @builtin(global_invocation_id) invocation_id: vec3<u32>,
    @builtin(num_workgroups) num_workgroups: vec3<u32>
) {
    let id = id(invocation_id, num_workgroups);

    particles[id].position += particles[id].velocity * delta_time;

    // Wrap position if it goes out of the viewport bounds
    if particles[id].position.x < 0.0 {
        particles[id].position.x += particle_config.world_width;
    } else if particles[id].position.x > particle_config.world_width {
        particles[id].position.x -= particle_config.world_width;
    }

    if particles[id].position.y < 0.0 {
        particles[id].position.y += particle_config.world_height;
    } else if particles[id].position.y > particle_config.world_height {
        particles[id].position.y -= particle_config.world_height;
    }
}

@compute @workgroup_size(WORKGROUP_SIZE, 1, 1)
fn update_spatial_hash_grid(
    @builtin(global_invocation_id) invocation_id: vec3<u32>,
    @builtin(num_workgroups) num_workgroups: vec3<u32>
) {
    let id = id(invocation_id, num_workgroups);

    // Reset offsets
    spatial_offsets[id] = particle_config.n;

    // Update index buffer
    let index = id;
    let cell = get_cell(particles[index].position, particle_config.r_max);
    let hash = hash_cell(cell);
    let key = key_from_hash(hash, particle_config.n);
    spatial_indices[id] = SpatialIndex(index, hash, key);
}