#import "particle.wgsl"::{Particles,ParticleConfig,HashGridEntry}
#import "utils.wgsl"::{force, getCell2D, keyFromHash, hashCell2D, offsets_2d, getOffset}

@group(0) @binding(0)
var<storage, read_write> particles: array<Particles>;

@group(0) @binding(1)
var<uniform> particle_config: ParticleConfig;

@group(0) @binding(2)
var<storage, read> attraction_matrix: array<f32>;

@group(0) @binding(3)
var<uniform> delta_time: f32;

@group(0) @binding(4)
var<storage, read_write> spatial_indices: array<HashGridEntry>;

@group(0) @binding(5)
var<storage, read_write> spatial_offsets: array<u32>;

const WIDTH: f32 = #{WIDTH}.0;
const HEIGHT: f32 = #{HEIGHT}.0;
const WORKGROUP_SIZE: u32 = #{WORKGROUP_SIZE};

fn hash(value: u32) -> u32 {
    var state = value;
    state = state ^ 2747636419u;
    state = state * 2654435769u;
    state = state ^ state >> 16u;
    state = state * 2654435769u;
    state = state ^ state >> 16u;
    state = state * 2654435769u;
    return state;
}

fn randomFloat(value: u32) -> f32 {
    return f32(hash(value)) / 4294967295.0;
}

fn hash2(value: u32) -> u32 {
    var state = value;
    state = state ^ 3104587013u;
    state = state * 1654435769u;
    state = state ^ state >> 16u;
    state = state * 2301324115u;
    state = state ^ state >> 16u;
    state = state * 2351435769u;
    return state;
}

fn randomFloat2(value: u32) -> f32 {
    return f32(hash2(value)) / 4294967295.0;
}

fn randomFloat3(value: u32) -> f32 {
    return randomFloat(value) * 2.0 - 1.0;
}

fn randomFloat4(value: u32) -> f32 {
    return randomFloat2(value) * 2.0 - 1.0;
}

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
    let origin_cell = getCell2D(particles[id].position, particle_config.r_max);
    let sqr_radius = f32(particle_config.r_max * particle_config.r_max);
    let velocity = particles[id].velocity;

    for (var i: i32 = 0; i < 9; i++) {
        var offset = getOffset(i);

        let hash = hashCell2D(origin_cell + offset);
        let key = keyFromHash(hash, particle_config.n);
        var curr_index = spatial_offsets[key];

        while curr_index < particle_config.n {
            let index_data = spatial_indices[curr_index];
            curr_index++;
            if index_data.key != key {
                break;
            } // Exit if no longer looking at the correct bin
            if index_data.hash != hash {
                continue;
            } // Skip if hash does not match

            let neighbour_index = index_data.original_index;
            if neighbour_index == id {
                continue;
            } // Skip if looking at self

            let neighbour = particles[neighbour_index];
            let offset_to_neighbour = neighbour.position - particles[id].position;
            let sqr_dst_to_neighbour = dot(offset_to_neighbour, offset_to_neighbour);

            if sqr_dst_to_neighbour > sqr_radius {
                continue;
            } // Skip if not within radius

            let r = sqrt(sqr_dst_to_neighbour);
            let a = attraction_matrix[particles[id].particle_type * particle_config.m + neighbour.particle_type];

            if r > 0.0 && r < particle_config.r_max {
                let f = force(r / particle_config.r_max, a);
                total_force += offset_to_neighbour / r * f * particle_config.r_max * particle_config.force_factor;
            }

            //let f = force(dst / particle_config.r_max, a);
            //total_force += vec2<f32>(rx / dst * f, ry / dst * f) * particle_config.r_max * particle_config.force_factor;

            //let neighbour_velocity = particles[neighbour_index].velocity;
            //total_force += neighbour_velocity - velocity;
        }
    }

    //particles[id].velocity = (particles[id].velocity + total_force * particle_config.dt) * particle_config.friction_factor * delta_time;
    particles[id].velocity = (particles[id].velocity + total_force * particle_config.dt) * particle_config.friction_factor ;
}

    // for (var i: u32 = 0; i < particle_config.n; i++) {
    //     if i == id {
    //         continue; // Skip self
    //     }
    //     let other = particles[i];
    //     let rx = other.position.x - particles[id].position.x;
    //     let ry = other.position.y - particles[id].position.y;
    //     let r = length(vec2<f32>(rx, ry));

    //     let a = attraction_matrix[particles[id].particle_type * particle_config.m + other.particle_type];

    //     if r > 0.0 && r < particle_config.r_max {
    //         let f = force(r / particle_config.r_max, a);
    //         total_force += vec2<f32>(rx / r * f, ry / r * f) * particle_config.r_max * particle_config.force_factor;
    //     }
    // }

    // particles[id].velocity = (particles[id].velocity + total_force * particle_config.dt) * particle_config.friction_factor;

@compute @workgroup_size(WORKGROUP_SIZE, 1, 1)
fn update_positions(
    @builtin(global_invocation_id) invocation_id: vec3<u32>,
    @builtin(num_workgroups) num_workgroups: vec3<u32>
) {
    let id = id(invocation_id, num_workgroups);

    //particles[id].position += particles[id].velocity * delta_time;
    particles[id].position += particles[id].velocity;

    // Wrap position if it goes out of the window bounds
    if particles[id].position.x < 0.0 {
        particles[id].position.x += WIDTH;
    } else if particles[id].position.x > WIDTH {
        particles[id].position.x -= WIDTH;
    }

    if particles[id].position.y < 0.0 {
        particles[id].position.y += HEIGHT;
    } else if particles[id].position.y > HEIGHT {
        particles[id].position.y -= HEIGHT;
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
    let cell = getCell2D(particles[index].position, particle_config.r_max);
    let hash = hashCell2D(cell);
    let key = keyFromHash(hash, particle_config.n);
    spatial_indices[id] = HashGridEntry(index, hash, key);
}